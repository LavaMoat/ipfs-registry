//! Validation for namespace and package identifiers.
use unicode_security::{
    GeneralSecurityProfile,
    restriction_level::{RestrictionLevelDetection, RestrictionLevel},
};

/// Validate an identifier.
pub fn validate(s: &str) -> bool {
    for c in s.chars() {
        if !c.is_ascii_digit() {
            if c != '-' && !c.is_alphabetic() {
                return false;
            }
        }

        // Unicode security
        if !c.identifier_allowed() {
            return false;
        }
    }

    // Single script
    if !s.check_restriction_level(
        RestrictionLevel::SingleScript) {
        return false;
    }
    true
}

#[cfg(test)]
mod test {
    use super::validate;

    /// Invisible characters.
    const INVISIBLES: &[char] = &[
        '\u{0009}',  // CHARACTER TABULATION
        '\u{0020}',  // SPACE
        '\u{00A0}',  // NO-BREAK SPACE
        '\u{00AD}',  // SOFT HYPHEN
        '\u{034F}',  // COMBINING GRAPHEME JOINER
        '\u{061C}',  // ARABIC LETTER MARK
        '\u{115F}',  // HANGUL CHOSEONG FILLER
        '\u{1160}',  // HANGUL JUNGSEONG FILLER
        '\u{17B4}',  // KHMER VOWEL INHERENT AQ
        '\u{17B5}',  // KHMER VOWEL INHERENT AA
        '\u{180E}',  // MONGOLIAN VOWEL SEPARATOR
        '\u{2000}',  // EN QUAD
        '\u{2001}',  // EM QUAD
        '\u{2002}',  // EN SPACE
        '\u{2003}',  // EM SPACE
        '\u{2004}',  // THREE-PER-EM SPACE
        '\u{2005}',  // FOUR-PER-EM SPACE
        '\u{2006}',  // SIX-PER-EM SPACE
        '\u{2007}',  // FIGURE SPACE
        '\u{2008}',  // PUNCTUATION SPACE
        '\u{2009}',  // THIN SPACE
        '\u{200A}',  // HAIR SPACE
        '\u{200B}',  // ZERO WIDTH SPACE
        '\u{200C}',  // ZERO WIDTH NON-JOINER
        '\u{200D}',  // ZERO WIDTH JOINER
        '\u{200E}',  // LEFT-TO-RIGHT MARK
        '\u{200F}',  // RIGHT-TO-LEFT MARK
        '\u{202F}',  // NARROW NO-BREAK SPACE
        '\u{205F}',  // MEDIUM MATHEMATICAL SPACE
        '\u{2060}',  // WORD JOINER
        '\u{2061}',  // FUNCTION APPLICATION
        '\u{2062}',  // INVISIBLE TIMES
        '\u{2063}',  // INVISIBLE SEPARATOR
        '\u{2064}',  // INVISIBLE PLUS
        '\u{206A}',  // INHIBIT SYMMETRIC SWAPPING
        '\u{206B}',  // ACTIVATE SYMMETRIC SWAPPING
        '\u{206C}',  // INHIBIT ARABIC FORM SHAPING
        '\u{206D}',  // ACTIVATE ARABIC FORM SHAPING
        '\u{206E}',  // NATIONAL DIGIT SHAPES
        '\u{206F}',  // NOMINAL DIGIT SHAPES
        '\u{3000}',  // IDEOGRAPHIC SPACE
        '\u{2800}',  // BRAILLE PATTERN BLANK
        '\u{3164}',  // HANGUL FILLER
        '\u{FEFF}',  // ZERO WIDTH NO-BREAK SPACE
        '\u{FFA0}',  // HALFWIDTH HANGUL FILLER
        '\u{1D159}', // MUSICAL SYMBOL NULL NOTEHEAD
        '\u{1D173}', // MUSICAL SYMBOL BEGIN BEAM
        '\u{1D174}', // MUSICAL SYMBOL END BEAM
        '\u{1D175}', // MUSICAL SYMBOL BEGIN TIE
        '\u{1D176}', // MUSICAL SYMBOL END TIE
        '\u{1D177}', // MUSICAL SYMBOL BEGIN SLUR
        '\u{1D178}', // MUSICAL SYMBOL END SLUR
        '\u{1D179}', // MUSICAL SYMBOL BEGIN PHRASE
        '\u{1D17A}', // MUSICAL SYMBOL END PHRASE
    ];

    #[test]
    fn validate_identifier() {
        // Valid identifier (ASCII)
        assert!(validate("foo-bar-qux"));

        // Valid identifier (Unicode)
        assert!(validate("〆切"));

        // Valid identifier
        assert!(validate("0x1fc770ac21067a04f83101ebf19a670db9e3eb21"));

        // Punctuation denied
        assert!(!validate("!"));

        // Control character denied
        assert!(!validate("\r"));

        // Invisible characters denied
        for c in INVISIBLES {
            assert!(!validate(&c.to_string()));
        }

        // Emoji is denied
        assert!(!validate("❤️"));

        // Unicode security
        assert!(!validate("µ"));

        // Mixed scripts
        // See: https://www.unicode.org/reports/tr39/#def-single-script
        assert!(!validate("Сirсlе"));
    }
}
