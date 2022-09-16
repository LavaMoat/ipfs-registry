use semver::{Op, Version, VersionReq};

use sqlx::{
    sqlite::SqliteArguments, Arguments, QueryBuilder, Sqlite, SqlitePool,
};
use web3_address::ethereum::Address;

use ipfs_registry_core::{
    confusable_skeleton, Namespace, ObjectKey, PackageKey, PackageName,
    Pointer,
};

use crate::{
    model::{NamespaceModel, Pager, VersionIncludes},
    value_objects::*,
    Error, Result,
};

pub struct PackageModel;

impl PackageModel {
    /// List packages for a namespace.
    pub async fn list_packages(
        pool: &SqlitePool,
        namespace: &Namespace,
        pager: &Pager,
        versions: VersionIncludes,
    ) -> Result<ResultSet<PackageRecord>> {
        // Check the namespace exists
        let namespace_record = NamespaceModel::find_by_name(pool, namespace)
            .await?
            .ok_or_else(|| Error::UnknownNamespace(namespace.clone()))?;

        let mut args: SqliteArguments = Default::default();
        args.add(namespace_record.namespace_id);
        args.add(pager.limit);
        args.add(pager.offset);

        let sql = format!(
            r#"
            SELECT
                (SELECT COUNT(package_id) FROM packages) as count,
                namespace_id,
                package_id,
                created_at,
                name,
                deprecated
            FROM packages
            WHERE namespace_id = ?
            --GROUP BY package_id
            ORDER BY name {}
            LIMIT ? OFFSET ?"#,
            pager.sort
        );

        let records = sqlx::query_as_with::<_, PackageRecord, _>(&sql, args)
            .fetch_all(pool)
            .await?;

        let packages = match versions {
            VersionIncludes::Latest => {
                let mut packages = Vec::with_capacity(records.len());
                for mut package in records {
                    let latest =
                        PackageModel::find_latest(pool, &package, false)
                            .await?
                            .ok_or(Error::NoPackageVersion)?;
                    package.versions.count = latest.count;
                    package.versions.records = vec![latest];
                    packages.push(package);
                }
                packages
            }
            VersionIncludes::None => records,
        };

        Ok(packages.into_result_set())
    }

    /// List versions of a package.
    pub async fn list_versions(
        pool: &SqlitePool,
        namespace: &Namespace,
        name: &PackageName,
        pager: &Pager,
    ) -> Result<ResultSet<VersionRecord>> {
        // Find the namespace
        let namespace_record = NamespaceModel::find_by_name(pool, namespace)
            .await?
            .ok_or_else(|| Error::UnknownNamespace(namespace.clone()))?;

        // Find the package
        let package_record = PackageModel::find_by_name(
            pool,
            namespace_record.namespace_id,
            name,
        )
        .await?
        .ok_or_else(|| Error::UnknownPackage(name.to_owned()))?;

        let mut args: SqliteArguments = Default::default();
        args.add(package_record.package_id);
        args.add(pager.limit);
        args.add(pager.offset);

        let sql = format!(
            r#"
            SELECT
                (SELECT COUNT(version_id) FROM versions) as count,
                version_id,
                publisher_id,
                package_id,
                major,
                minor,
                patch,
                pre,
                build,
                -- package,
                content_id,
                pointer_id,
                signature,
                checksum,
                yanked,
                created_at
            FROM versions
            WHERE package_id = ?
            --GROUP BY version_id
            ORDER BY major {}, minor {}, patch {}, pre {}, build {}
            LIMIT ? OFFSET ?"#,
            pager.sort, pager.sort, pager.sort, pager.sort, pager.sort,
        );

        let records = sqlx::query_as_with::<_, VersionRecord, _>(&sql, args)
            .fetch_all(pool)
            .await?;

        Ok(records.into_result_set())
    }

    /// Find a package version by package key.
    pub async fn find_by_key(
        pool: &SqlitePool,
        package_key: &PackageKey,
    ) -> Result<(
        Option<NamespaceRecord>,
        Option<PackageRecord>,
        Option<VersionRecord>,
    )> {
        match package_key {
            PackageKey::Pointer(namespace, name, version) => {
                let namespace_record =
                    NamespaceModel::find_by_name(pool, namespace)
                        .await?
                        .ok_or_else(|| {
                            Error::UnknownNamespace(namespace.clone())
                        })?;
                let (package_record, version_record) =
                    PackageModel::find_by_name_version(
                        pool,
                        namespace_record.namespace_id,
                        name,
                        version,
                    )
                    .await?;

                Ok((Some(namespace_record), package_record, version_record))
            }
            PackageKey::Cid(cid) => {
                let mut args: SqliteArguments = Default::default();
                args.add(cid.to_string());

                let record = sqlx::query_as_with::<_, VersionRecord, _>(
                    r#"SELECT * FROM versions WHERE content_id = ?"#,
                    args,
                )
                .fetch_optional(pool)
                .await?;

                let package_record = if let Some(record) = &record {
                    PackageModel::find_package_by_id(pool, record.package_id)
                        .await?
                } else {
                    None
                };

                let namespace_record = if let Some(record) = &package_record {
                    NamespaceModel::find_namespace_by_id(
                        pool,
                        record.namespace_id,
                    )
                    .await?
                } else {
                    None
                };

                Ok((namespace_record, package_record, record))
            }
        }
    }

    /// Find multiple packages by name.
    pub async fn find_many_by_name<'a>(
        pool: &SqlitePool,
        namespace_id: i64,
        packages: Vec<&'a PackageName>,
    ) -> Result<Vec<(&'a PackageName, Option<PackageRecord>)>> {
        let mut records = Vec::new();
        for name in packages {
            records.push((
                name,
                PackageModel::find_by_name(pool, namespace_id, name).await?,
            ));
        }
        Ok(records)
    }

    /// Find a package by name.
    pub async fn find_by_name(
        pool: &SqlitePool,
        namespace_id: i64,
        name: &PackageName,
    ) -> Result<Option<PackageRecord>> {
        let skeleton = confusable_skeleton(name.as_str());
        let mut args: SqliteArguments = Default::default();
        args.add(namespace_id);
        args.add(&skeleton);

        let record = sqlx::query_as_with::<_, PackageRecord, _>(
            r#"
                SELECT
                    namespace_id,
                    package_id,
                    created_at,
                    name,
                    deprecated
                FROM packages
                WHERE namespace_id = ? AND skeleton = ?
            "#,
            args,
        )
        .fetch_optional(pool)
        .await?;

        Ok(record)
    }

    fn with_operator(
        builder: &mut QueryBuilder<Sqlite>,
        args: &mut SqliteArguments,
        column: &str,
        operator: &str,
        combined: String,
    ) {
        let op = format!(" {} ", operator);
        builder.push(column);
        builder.push(&op);
        builder.push_bind(combined.to_string());
        args.add(combined);
    }

    fn version_req_condition(
        builder: &mut QueryBuilder<Sqlite>,
        args: &mut SqliteArguments,
        versions: &VersionReq,
    ) {
        let len = versions.comparators.len();
        for (index, comparator) in versions.comparators.iter().enumerate() {
            let major = comparator.major as i64;
            let minor = comparator.minor.unwrap_or(0) as i64;
            let patch = comparator.patch.unwrap_or(0) as i64;
            let pre = comparator.pre.to_string();
            let (combined, column) = if comparator.minor.is_none() {
                (format!("{}", major), "major")
            } else if comparator.minor.is_some() && comparator.patch.is_none()
            {
                (format!("{}{}", major, minor), "major_minor")
            } else if comparator.patch.is_some() {
                (format!("{}{}{}", major, minor, patch), "major_minor_patch")
            } else {
                (format!("{}{}{}{}", major, minor, patch, pre), "version")
            };

            builder.push("(");
            match comparator.op {
                Op::Exact => {
                    PackageModel::with_operator(
                        builder, args, column, "=", combined,
                    );
                }
                Op::Greater => {
                    PackageModel::with_operator(
                        builder, args, column, ">", combined,
                    );
                }
                Op::GreaterEq => {
                    PackageModel::with_operator(
                        builder, args, column, ">=", combined,
                    );
                }
                Op::Less => {
                    PackageModel::with_operator(
                        builder, args, column, "<", combined,
                    );
                }
                Op::LessEq => {
                    PackageModel::with_operator(
                        builder, args, column, "<=", combined,
                    );
                }
                Op::Tilde => {
                    if comparator.patch.is_none() {
                        PackageModel::with_operator(
                            builder, args, column, "=", combined,
                        );
                    } else {
                        PackageModel::with_operator(
                            builder, args, column, ">=", combined,
                        );

                        builder.push(" AND ");

                        let upper_bound =
                            format!("{}{}{}", major, minor + 1, 0);

                        PackageModel::with_operator(
                            builder,
                            args,
                            column,
                            "<",
                            upper_bound,
                        );
                    }
                }
                Op::Wildcard => {
                    if comparator.minor.is_none()
                        || (comparator.minor.is_some()
                            && comparator.patch.is_none())
                    {
                        PackageModel::with_operator(
                            builder, args, column, "=", combined,
                        );
                    }
                }
                Op::Caret => {
                    if comparator.patch.is_some() {
                        // ^0.J.K (for J>0) — equivalent to >=0.J.K, <0.(J+1).0
                        if major == 0 && minor > 0 {
                            PackageModel::with_operator(
                                builder, args, column, ">=", combined,
                            );
                            builder.push(" AND ");
                            let upper_bound =
                                format!("{}{}{}", major, minor + 1, 0);
                            PackageModel::with_operator(
                                builder,
                                args,
                                column,
                                "<",
                                upper_bound,
                            );
                        } else if major > 0 {
                            PackageModel::with_operator(
                                builder, args, column, ">=", combined,
                            );
                            builder.push(" AND ");
                            let upper_bound =
                                format!("{}{}{}", major + 1, 0, 0);
                            PackageModel::with_operator(
                                builder,
                                args,
                                column,
                                "<",
                                upper_bound,
                            );
                        } else {
                            PackageModel::with_operator(
                                builder, args, column, "=", combined,
                            );
                        }
                    } else {
                        PackageModel::with_operator(
                            builder, args, column, "=", combined,
                        );
                    }
                }
                _ => {}
            }
            builder.push(")");

            if index < len - 1 {
                builder.push(" OR ");
            }
        }
    }

    /// Find versions of a package that match the request.
    pub async fn find_versions(
        pool: &SqlitePool,
        namespace: &Namespace,
        name: &PackageName,
        versions: &VersionReq,
        pager: &Pager,
    ) -> Result<ResultSet<VersionRecord>> {
        // Find the namespace
        let namespace_record = NamespaceModel::find_by_name(pool, namespace)
            .await?
            .ok_or_else(|| Error::UnknownNamespace(namespace.clone()))?;

        // Find the package
        let package_record = PackageModel::find_by_name(
            pool,
            namespace_record.namespace_id,
            name,
        )
        .await?
        .ok_or_else(|| Error::UnknownPackage(name.to_owned()))?;

        let mut args: SqliteArguments = Default::default();
        args.add(package_record.package_id);

        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
                SELECT
                    (SELECT COUNT(version_id) FROM versions) as count,
                    version_id,
                    publisher_id,
                    package_id,
                    major,
                    minor,
                    patch,
                    pre,
                    build,
                    (major || minor) as major_minor,
                    (major || minor || patch) as major_minor_patch,
                    (major || minor || patch || pre) as version,
                    package,
                    content_id,
                    pointer_id,
                    signature,
                    checksum,
                    yanked,
                    created_at
                FROM versions
                WHERE package_id = "#,
        );
        builder.push_bind(package_record.package_id);
        builder.push(
            r#"
            GROUP BY version_id
            HAVING "#,
        );

        PackageModel::version_req_condition(
            &mut builder,
            &mut args,
            versions,
        );

        args.add(pager.limit);
        args.add(pager.offset);

        let ordering = format!(
            "major {}, minor {}, patch {}, pre {}, build {}",
            pager.sort, pager.sort, pager.sort, pager.sort, pager.sort
        );

        builder.push(format!(
            r#"
                ORDER BY {}
                LIMIT "#,
            ordering
        ));
        builder.push_bind(pager.limit);
        builder.push(r#" OFFSET "#);
        builder.push_bind(pager.offset);

        let sql = builder.into_sql();

        let records = sqlx::query_as_with::<_, VersionRecord, _>(&sql, args)
            .fetch_all(pool)
            .await?;

        Ok(records.into_result_set())
    }

    /// Find latest version by namespace and package name.
    pub async fn find_latest_by_name(
        pool: &SqlitePool,
        namespace: &Namespace,
        name: &PackageName,
        include_prerelease: bool,
    ) -> Result<Option<VersionRecord>> {
        // Find the namespace
        let namespace_record = NamespaceModel::find_by_name(pool, namespace)
            .await?
            .ok_or_else(|| Error::UnknownNamespace(namespace.clone()))?;

        // Find the package
        let package_record = PackageModel::find_by_name(
            pool,
            namespace_record.namespace_id,
            name,
        )
        .await?
        .ok_or_else(|| Error::UnknownPackage(name.to_owned()))?;

        PackageModel::find_latest(pool, &package_record, include_prerelease)
            .await
    }

    /// Find latest version of a package.
    pub async fn find_latest(
        pool: &SqlitePool,
        package_record: &PackageRecord,
        include_prerelease: bool,
    ) -> Result<Option<VersionRecord>> {
        let mut args: SqliteArguments = Default::default();
        args.add(package_record.package_id);

        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
                SELECT
                    (SELECT COUNT(version_id) FROM versions) as count,
                    version_id,
                    publisher_id,
                    package_id,
                    major,
                    minor,
                    patch,
                    pre,
                    build,
                    package,
                    content_id,
                    pointer_id,
                    signature,
                    checksum,
                    yanked,
                    created_at
                FROM versions WHERE package_id =
            "#,
        );
        builder.push_bind(package_record.package_id);

        if include_prerelease {
            builder.push(
                r#"
                    ORDER BY major DESC, minor DESC, patch DESC, pre DESC, build DESC
                    LIMIT 1
                "#);
        } else {
            builder.push(
                r#"
                    AND pre = ""
                    ORDER BY major DESC, minor DESC, patch DESC
                    LIMIT 1
                "#,
            );
        }

        let sql = builder.into_sql();
        let record = sqlx::query_as_with::<_, VersionRecord, _>(&sql, args)
            .fetch_optional(pool)
            .await?;

        Ok(record)
    }

    /// Find a package by name and version.
    pub async fn find_by_name_version(
        pool: &SqlitePool,
        namespace_id: i64,
        name: &PackageName,
        version: &Version,
    ) -> Result<(Option<PackageRecord>, Option<VersionRecord>)> {
        if let Some(package_record) =
            PackageModel::find_by_name(pool, namespace_id, name).await?
        {
            let mut args: SqliteArguments = Default::default();
            args.add(package_record.package_id);
            args.add(version.major as i64);
            args.add(version.minor as i64);
            args.add(version.patch as i64);
            args.add(version.pre.to_string());
            args.add(version.build.to_string());

            let record = sqlx::query_as_with::<_, VersionRecord, _>(
                r#"
                    SELECT * FROM versions
                    WHERE package_id = ? AND major = ? AND minor = ? AND patch = ? AND pre = ? AND build = ?
                "#,
                args
            )
            .fetch_optional(pool)
            .await?;

            Ok((Some(package_record), record))
        } else {
            Ok((None, None))
        }
    }

    /// Find a package by id.
    pub async fn find_package_by_id(
        pool: &SqlitePool,
        package_id: i64,
    ) -> Result<Option<PackageRecord>> {
        let mut args: SqliteArguments = Default::default();
        args.add(package_id);
        let record = sqlx::query_as_with::<_, PackageRecord, _>(
            r#"SELECT * FROM packages WHERE package_id = ?"#,
            args,
        )
        .fetch_optional(pool)
        .await?;
        Ok(record)
    }

    /// Find or insert a new package.
    pub async fn find_or_insert(
        pool: &SqlitePool,
        namespace_id: i64,
        name: &PackageName,
    ) -> Result<PackageRecord> {
        if let Some(record) =
            PackageModel::find_by_name(pool, namespace_id, name).await?
        {
            Ok(record)
        } else {
            let mut builder = QueryBuilder::new(
                r#"
                    INSERT INTO packages ( namespace_id, name, skeleton, created_at )
                    VALUES (
                "#,
            );

            let skeleton = confusable_skeleton(name.as_str());
            let mut separated = builder.separated(", ");
            separated.push_bind(namespace_id);
            separated.push_bind(name.as_str());
            separated.push_bind(&skeleton);
            builder.push(", datetime('now') )");

            let id = builder.build().execute(pool).await?.last_insert_rowid();

            let record = PackageModel::find_package_by_id(pool, id)
                .await?
                .ok_or(Error::InsertFetch(id))?;

            Ok(record)
        }
    }

    /// Add a package version to a namespace.
    ///
    /// If a package already exists for the given name
    /// and version return `None`.
    pub async fn insert(
        pool: &SqlitePool,
        publisher_record: &PublisherRecord,
        namespace_record: &NamespaceRecord,
        _publisher: &Address,
        pointer: &Pointer,
    ) -> Result<i64> {
        let name = &pointer.definition.artifact.package.name;
        let version = &pointer.definition.artifact.package.version;

        let pointer_id = pointer.definition.artifact.pointer_id();
        let content_id = pointer.definition.objects.iter().find_map(|o| {
            if let ObjectKey::Cid(cid) = o {
                Some(cid.to_string())
            } else {
                None
            }
        });

        // Find or insert the package
        let package = serde_json::to_string(&pointer.package)?;

        //let version = version.to_string();
        let package_record = PackageModel::find_or_insert(
            pool,
            namespace_record.namespace_id,
            name,
        )
        .await?;

        // Insert the package version
        let mut builder = QueryBuilder::new(
            r#"
                INSERT INTO versions ( publisher_id, package_id, major, minor, patch, pre, build, package, content_id, pointer_id, signature, checksum, created_at )
                VALUES (
            "#,
        );
        let mut separated = builder.separated(", ");
        separated.push_bind(publisher_record.publisher_id);
        separated.push_bind(package_record.package_id);
        separated.push_bind(version.major as i64);
        separated.push_bind(version.minor as i64);
        separated.push_bind(version.patch as i64);
        separated.push_bind(version.pre.to_string());
        separated.push_bind(version.build.to_string());
        separated.push_bind(package);
        separated.push_bind(content_id);
        separated.push_bind(pointer_id);
        separated.push_bind(pointer.definition.signature.value.to_vec());
        separated.push_bind(pointer.definition.checksum.to_vec());
        builder.push(", datetime('now') )");

        let id = builder.build().execute(pool).await?.last_insert_rowid();

        Ok(id)
    }

    /// Assert publishing is ok by checking a package
    /// with the given name and version does not already exist, the
    /// target version is ahead of the latest published version
    /// and verify access control permissions.
    pub async fn can_publish_package(
        pool: &SqlitePool,
        address: &Address,
        namespace_record: &NamespaceRecord,
        name: &PackageName,
        version: Option<&Version>,
    ) -> Result<Option<PackageRecord>> {
        let not_owner = address != &namespace_record.owner;
        let user = namespace_record
            .publishers
            .iter()
            .find(|u| &u.address == address);
        let is_restricted = if let (Some(user), true) = (user, not_owner) {
            !user.restrictions.is_empty()
        } else {
            false
        };

        // Not the owner and no user found for the namespace
        // so access is denied
        if not_owner && user.is_none() {
            return Err(Error::Unauthorized(*address));
        }

        let package_record = if let Some(version) = version {
            // Check the package / version does not already exist
            let (package_record, version_record) =
                PackageModel::find_by_name_version(
                    pool,
                    namespace_record.namespace_id,
                    name,
                    version,
                )
                .await?;
            if version_record.is_some() {
                return Err(Error::PackageExists(
                    namespace_record.name.clone(),
                    name.clone(),
                    version.clone(),
                ));
            }

            package_record
        } else {
            PackageModel::find_by_name(
                pool,
                namespace_record.namespace_id,
                name,
            )
            .await?
        };

        if let Some(package_record) = &package_record {
            // Package already exists and the user is restricted
            // so verify the user can publish to the target package
            if let (Some(user), true) = (user, is_restricted) {
                let can_publish =
                    user.restrictions.iter().any(|package_id| {
                        package_id == &package_record.package_id
                    });
                if !can_publish {
                    return Err(Error::Unauthorized(*address));
                }
            }

            if let Some(version) = version {
                // Verify the version to publish is ahead of the latest version
                if let Some(latest) = PackageModel::find_latest_by_name(
                    pool,
                    &namespace_record.name,
                    name,
                    true,
                )
                .await?
                {
                    if version <= &latest.version {
                        return Err(Error::VersionNotAhead(
                            version.clone(),
                            latest.version,
                        ));
                    }
                }
            }
        } else {
            // No existing package record so this is the first
            // publish for the package, restricted users should
            // be denied access
            if let (Some(_), true) = (user, is_restricted) {
                return Err(Error::Unauthorized(*address));
            }
        }

        Ok(package_record)
    }

    /// Mark a package as deprecated.
    pub async fn deprecate(
        pool: &SqlitePool,
        address: &Address,
        namespace: &Namespace,
        package: &PackageName,
        message: &str,
    ) -> Result<()> {
        let (_, namespace_record) = NamespaceModel::can_access_namespace(
            pool,
            &address,
            &namespace,
        )
        .await?;

        let package_record =
            PackageModel::find_by_name(pool, namespace_record.namespace_id, package).await?;
        let package_record =
            package_record.ok_or(Error::UnknownPackage(package.clone()))?;

        PackageModel::can_publish_package(
            pool,
            address,
            &namespace_record,
            &package_record.name,
            None,
        )
        .await?;

        let mut builder =
            QueryBuilder::<Sqlite>::new("UPDATE packages SET deprecated = ");
        builder.push_bind(message);
        builder.push("WHERE package_id = ");
        builder.push_bind(package_record.package_id);

        let mut args: SqliteArguments = Default::default();
        args.add(message);
        args.add(package_record.package_id);

        let sql = builder.into_sql();
        sqlx::query_with::<_, _>(&sql, args).execute(pool).await?;

        Ok(())
    }

    /// Yank a package.
    pub async fn yank(
        pool: &SqlitePool,
        address: &Address,
        id: &PackageKey,
        message: &str,
    ) -> Result<()> {
        let (namespace_record, package_record, version_record) =
            PackageModel::find_by_key(pool, id).await?;

        let package_record =
            package_record.ok_or(Error::UnknownPackageKey(id.clone()))?;
        let version_record =
            version_record.ok_or(Error::UnknownPackageKey(id.clone()))?;

        // Should have namespace if we have version record
        let namespace_record = namespace_record.unwrap();

        NamespaceModel::can_access_namespace(
            pool,
            &address,
            &namespace_record.name,
        )
        .await?;

        PackageModel::can_publish_package(
            pool,
            address,
            &namespace_record,
            &package_record.name,
            None,
        )
        .await?;

        let mut builder =
            QueryBuilder::<Sqlite>::new("UPDATE versions SET yanked = ");
        builder.push_bind(message);
        builder.push("WHERE version_id = ");
        builder.push_bind(version_record.version_id);

        let mut args: SqliteArguments = Default::default();
        args.add(message);
        args.add(version_record.version_id);

        let sql = builder.into_sql();

        sqlx::query_with::<_, _>(&sql, args).execute(pool).await?;

        Ok(())
    }
}
