use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::{fs, io};

use crate::internals::EnvError;

// this and the the below type may be superflouous
pub type EnvVar = String;

pub type EnvVal = String;

// if the above are not needed then change this to EnvMap = HashMap<String, String>
pub type EnvMap = HashMap<EnvVar, EnvVal>;

// internal mod to handle lexing and parsing
mod internals {
    use core::fmt;

    use super::{EnvMap, EnvVal, EnvVar};

    #[derive(Debug)]
    pub enum EnvToken {
        Character(char),
        AssignmentOperator,
        NewLine,
        Eof,
        Comment,
        DoubleQuoteMark,
        SingleQuoteMark,
        Whitespace,
    }

    /// tokenizes the given `.env` file into a Vec of Tokens
    pub fn lex_dot_env(file_contents: String) -> Vec<EnvToken> {
        file_contents
            .chars()
            .map(|c| match c {
                '=' => EnvToken::AssignmentOperator,
                ' ' => EnvToken::Whitespace,
                '#' => EnvToken::Comment,
                '\n' => EnvToken::NewLine,
                '"' => EnvToken::DoubleQuoteMark,
                '\'' => EnvToken::SingleQuoteMark,
                _ => EnvToken::Character(c),
            })
            .chain([EnvToken::Eof])
            .collect()
    }

    #[derive(Debug, PartialEq)]
    pub enum EnvError {
        UnexpectedToken {
            expected: String,
            found: String,
            line: u64,
            character: u64,
        },
        MissingAssignmentOperator {
            key: String,
            line: u64,
            character: u64,
        },
        ExpectedValueButFoundAssignment {
            line: u64,
            character: u64,
        },
        MissingKey {
            line: u64,
        },
        MissingValue {
            line: u64,
        },
        FoundOnlyKey {
            line: u64,
        },
        UnclosedValue {
            line: u64,
        },
    }

    impl fmt::Display for EnvError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                EnvError::UnexpectedToken {
                    expected,
                    found,
                    line,
                    character,
                } => write!(
                    f,
                    "Unexpected token: expected {expected} but found '{found}' at line {line}, character {character}",
                ),
                EnvError::MissingAssignmentOperator {
                    key,
                    line,
                    character,
                } => write!(
                    f,
                    "Missing assignment operator for key '{key}' on line {line}, character {character}",
                ),
                EnvError::ExpectedValueButFoundAssignment { line, character } => write!(
                    f,
                    "Expected value but found assignment operator at line {line}, character {character}"
                ),
                EnvError::MissingKey { line } => write!(f, "Key missing on line {line}"),
                EnvError::MissingValue { line } => write!(f, "Value missing on line {line}"),
                EnvError::FoundOnlyKey { line } => write!(
                    f,
                    "Only found key on line {line}, expected assignment operator and value"
                ),
                EnvError::UnclosedValue { line } => {
                    write!(f, "Key or value was not closed from line {line}")
                }
            }
        }
    }

    /// reads the Vec of Tokens into a valid EnvMap and returns an error
    /// for specific errors
    pub fn parse_dot_env(tokens: Vec<EnvToken>) -> Result<EnvMap, EnvError> {
        let mut new_env_map: EnvMap = EnvMap::new();
        let mut line_counter: u64 = 1;
        let mut character_counter: u64 = 1;
        let mut current_key: String = EnvVar::new();
        let mut current_value: String = EnvVal::new();
        let mut expecting_key: bool = true;
        let mut expecting_value: bool = false;
        let mut in_a_comment: bool = false;
        let mut encountered_assignment: bool = false;
        let mut in_single_quoted_string: bool = false;

        for token in tokens {
            match token {
                EnvToken::Character(c) => {
                    character_counter += 1;
                    if !in_a_comment {
                        if expecting_key {
                            current_key.push(c);
                            continue;
                        } else if expecting_value {
                            current_value.push(c);
                            continue;
                        } else if !expecting_value {
                            // this case is when we finish parsing a value but get another character
                            return Err(EnvError::UnexpectedToken {
                                expected: "comment of new line".to_string(),
                                found: c.to_string(),
                                line: line_counter,
                                character: character_counter,
                            });
                        }
                    }
                }
                EnvToken::AssignmentOperator => {
                    if in_single_quoted_string && expecting_value {
                        current_value.push('=');
                        continue;
                    }

                    // this throws an error if we already know we're expecting a value
                    // but we get an '=' sign and not any characters.
                    // but if there's already content in the current value, we know that this equals sign
                    // is in the value itself.
                    // this should be changed though once we account for quotation marks
                    if !expecting_key && current_value.is_empty() {
                        return Err(EnvError::ExpectedValueButFoundAssignment {
                            line: line_counter,
                            character: character_counter,
                        });
                    }

                    if !current_key.is_empty()
                        && !current_value.is_empty()
                        && encountered_assignment
                        && !in_a_comment
                    {
                        // this should be modified when we add quoote handling
                        return Err(EnvError::ExpectedValueButFoundAssignment {
                            line: line_counter,
                            character: character_counter,
                        });
                    }

                    if !in_a_comment {
                        encountered_assignment = true;
                    }
                    if in_a_comment {
                        encountered_assignment = false;
                    }
                    expecting_key = false;
                    expecting_value = true;
                    character_counter += 1;
                }
                EnvToken::Whitespace => {
                    if in_single_quoted_string {
                        if expecting_value {
                            current_value.push(' ');
                        }
                        continue;
                    }

                    character_counter += 1;
                    if in_a_comment {
                        continue;
                    }
                    if current_key.is_empty() && expecting_key {
                        return Err(EnvError::UnexpectedToken {
                            expected: "key or comment symbol".to_string(),
                            found: " ".to_string(),
                            line: line_counter,
                            character: character_counter,
                        });
                    }
                    if expecting_key {
                        return Err(EnvError::UnexpectedToken {
                            expected: "key or comment symbol".to_string(),
                            found: " ".to_string(),
                            line: line_counter,
                            character: character_counter,
                        });
                    }
                    if expecting_value {
                        expecting_value = false;
                    }
                }
                EnvToken::Comment => {
                    if in_single_quoted_string {
                        if expecting_value {
                            current_value.push('#');
                            continue;
                        }
                    }
                    in_a_comment = true;
                }
                EnvToken::NewLine => {
                    if in_single_quoted_string {
                        current_value.push('\n');
                        continue;
                    }

                    // if there is not key or value, and if there's no assignment operator,
                    // then just reset and continue
                    if (current_key.is_empty() && current_value.is_empty())
                        && !encountered_assignment
                    {
                        expecting_key = true;
                        expecting_value = false;
                        current_key.clear();
                        current_value.clear();
                        line_counter += 1;
                        in_a_comment = false;
                        character_counter = 0;
                        encountered_assignment = false;
                        continue;
                    }

                    // if there's an assignment operator but not key and value, throw an error
                    if encountered_assignment {
                        if current_key.is_empty() {
                            return Err(EnvError::MissingKey { line: line_counter });
                        };
                        if current_value.is_empty() {
                            return Err(EnvError::MissingValue { line: line_counter });
                        };
                    }

                    // if there's no assignment operator, but a key was encountered, error
                    if (!current_key.is_empty() && current_value.is_empty())
                        && !encountered_assignment
                    {
                        return Err(EnvError::FoundOnlyKey { line: line_counter });
                    }

                    // we have a few things to do on the new line token
                    // first, check whether the key and value are not empty strings
                    // if either is empty, throw an error and report the line
                    // on which the error occured
                    if current_key.is_empty() && !current_value.is_empty() {
                        // throw error
                        // this 'or' condition could be broken up into multiple error returns though
                        return Err(EnvError::MissingKey { line: line_counter });
                    }

                    if !current_key.is_empty() && current_value.is_empty() {
                        return Err(EnvError::MissingValue { line: line_counter });
                    }

                    if !current_key.is_empty() && !current_value.is_empty() {
                        // if there is no error,
                        // add the key and value to the map (remember to clone)
                        new_env_map.insert(current_key.clone(), current_value.clone());
                    }

                    // and then reset the state to expect a key
                    expecting_key = true;
                    expecting_value = false;
                    current_key.clear();
                    current_value.clear();
                    in_a_comment = false;
                    line_counter += 1;
                    character_counter = 0;
                    encountered_assignment = false;
                    // and not expect a value,
                    // and the line_character counter
                    // as well as calling the .clear() method on
                    // each of those strings
                }
                EnvToken::Eof => {
                    if in_single_quoted_string {
                        return Err(EnvError::UnclosedValue { line: line_counter });
                    }

                    if !current_key.is_empty() && !current_value.is_empty() {
                        new_env_map.insert(current_key.clone(), current_value.clone());
                    }
                    // throw an error if there is a key or value missing its pair
                    if current_key.is_empty() && !current_value.is_empty() {
                        return Err(EnvError::MissingKey { line: line_counter });
                    }
                    if !current_key.is_empty() && current_value.is_empty() {
                        return Err(EnvError::MissingValue { line: line_counter });
                    }
                    break;
                }
                EnvToken::SingleQuoteMark => {
                    if in_single_quoted_string {
                        // end of the single quoted string is found and assert we are not expecting any more of the value
                        in_single_quoted_string = false;
                        expecting_value = false;
                        continue;
                    }

                    // quotes are not allowed in keys, so
                    // if expecting a key, throw an error
                    if !in_single_quoted_string {
                        if expecting_key {
                            return Err(EnvError::UnexpectedToken {
                                expected: "key or assignment operator".to_string(),
                                found: "single quote mark".to_string(),
                                line: line_counter,
                                character: character_counter,
                            });
                        }
                        in_single_quoted_string = true;
                    }
                }
                EnvToken::DoubleQuoteMark => {
                    if in_single_quoted_string {
                        if expecting_key {
                            return Err(EnvError::UnexpectedToken {
                                expected: "key or assignment operator".to_string(),
                                found: "double quote mark".to_string(),
                                line: line_counter,
                                character: character_counter,
                            });
                        }
                        if expecting_value {
                            current_value.push('"');
                        }
                    }
                    continue;
                }
            }
        }

        Ok(new_env_map)
    }
}
/// fully reads and parses a `.env` file to return a map of non-empty key-value pairs
pub fn process_dot_env(file_contents: String) -> Result<HashMap<String, String>, EnvError> {
    internals::parse_dot_env(internals::lex_dot_env(file_contents))
}

/// serializes a hash map to a file, overwriting it if it already exists.
pub fn serialize_new_env(file_name: String, hash_map: EnvMap) -> Result<String, io::Error> {
    let file = fs::File::create(file_name.clone())?;
    let mut writer = BufWriter::new(file);
    hash_map
        .iter()
        .try_for_each(|map| writer.write_all(format!("{}={}\n", map.0, map.1).as_bytes()))?;
    writer.flush()?;
    Ok(format!("serialized to {file_name}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        internals::{EnvToken, lex_dot_env},
        process_dot_env, serialize_new_env,
    };

    /// reads a simple vec of tokens that should not error
    #[test]
    fn simple_lex_dot_env() {
        let contents = "KEY=VAL\n# comment\n".to_string();
        let tokens = lex_dot_env(contents);
        let expected_tokens = vec![
            EnvToken::Character('K'),
            EnvToken::Character('E'),
            EnvToken::Character('Y'),
            EnvToken::AssignmentOperator,
            EnvToken::Character('V'),
            EnvToken::Character('A'),
            EnvToken::Character('L'),
            EnvToken::NewLine,
            EnvToken::Comment,
            EnvToken::Whitespace,
            EnvToken::Character('c'),
            EnvToken::Character('o'),
            EnvToken::Character('m'),
            EnvToken::Character('m'),
            EnvToken::Character('e'),
            EnvToken::Character('n'),
            EnvToken::Character('t'),
            EnvToken::NewLine,
            EnvToken::Eof,
        ];

        assert_eq!(format!("{:?}", tokens), format!("{:?}", expected_tokens))
    }

    /// reads a simple vec of tokens that should not error
    #[test]
    fn simple_single_quoted_lex_dot_env() {
        let contents = "KEY='VAL'\n# comment\n".to_string();
        let tokens = lex_dot_env(contents);
        let expected_tokens = vec![
            EnvToken::Character('K'),
            EnvToken::Character('E'),
            EnvToken::Character('Y'),
            EnvToken::AssignmentOperator,
            EnvToken::SingleQuoteMark,
            EnvToken::Character('V'),
            EnvToken::Character('A'),
            EnvToken::Character('L'),
            EnvToken::SingleQuoteMark,
            EnvToken::NewLine,
            EnvToken::Comment,
            EnvToken::Whitespace,
            EnvToken::Character('c'),
            EnvToken::Character('o'),
            EnvToken::Character('m'),
            EnvToken::Character('m'),
            EnvToken::Character('e'),
            EnvToken::Character('n'),
            EnvToken::Character('t'),
            EnvToken::NewLine,
            EnvToken::Eof,
        ];

        assert_eq!(format!("{:?}", tokens), format!("{:?}", expected_tokens))
    }

    /// reads a simple, well-formatted file that should not error
    #[test]
    fn read_simple_file() {
        let contents = fs::read_to_string("tests/Test.env").expect("error reading test env file");
        let test_map = process_dot_env(contents).expect("error processing env file");
        assert_eq!(test_map.get("Hello").unwrap(), "World")
    }

    /// note the lack of a value at line 1 character 5
    #[test]
    fn expect_missing_value_err() {
        let contents = "KEY=\n# comment\n".to_string();
        let test_map = process_dot_env(contents);

        match test_map {
            Err(crate::internals::EnvError::MissingValue { line }) => {
                assert_eq!(line, 1);
            }
            _ => panic!("Did not return correct error"),
        }
    }

    /// note the lack of a key at line one character 1
    #[test]
    fn expect_missing_key_err() {
        let contents = "=VAL\n# comment\n".to_string();
        let test_map = process_dot_env(contents);

        match test_map {
            Err(crate::internals::EnvError::MissingKey { line }) => {
                assert_eq!(line, 1);
            }
            _ => panic!("Did not return correct error"),
        }
    }

    /// NOTE the whitespace after the new line
    #[test]
    fn expect_unexpected_token_err() {
        let contents = "KEY=VAL\n # comment\n".to_string();
        let test_map = process_dot_env(contents);

        match test_map {
            Err(crate::internals::EnvError::UnexpectedToken {
                line, character, ..
            }) => {
                assert_eq!(line, 2);
                assert_eq!(character, 1);
            }
            _ => panic!("Did not return correct error"),
        }
    }

    /// expect an error that the single quote is never closed
    #[test]
    fn expect_unclosed_signel_quote_err() {
        let contents = "KEY='VAL\n # comment\n".to_string();
        let test_map = process_dot_env(contents);

        match test_map {
            Err(crate::internals::EnvError::UnclosedValue { line, .. }) => {
                assert_eq!(line, 1);
            }
            _ => panic!("Did not return correct error"),
        }
    }

    /// expect an error that the value is missing
    #[test]
    fn expect_empty_val_single_quote_err() {
        let contents = "KEY='' # same line comment \n # new line comment\n".to_string();
        let test_map = process_dot_env(contents);

        match test_map {
            Err(crate::internals::EnvError::MissingValue { line, .. }) => {
                assert_eq!(line, 1);
            }
            _ => panic!("Did not return correct error"),
        }
    }

    /// keys cannot have single or double quotes, only numbers, letters, and underscores,
    /// and must begin with a letter
    #[test]
    fn expect_unexpected_value_in_key_single_quote_err() {
        let contents = "'KEY'='value' # same line comment \n".to_string();
        let test_map = process_dot_env(contents);
        match test_map {
            Err(crate::internals::EnvError::UnexpectedToken { line, .. }) => {
                assert_eq!(line, 1);
            }
            _ => panic!("Did not return correct error"),
        }
    }

    /// do not expect an error parsing special characters in a quoted value
    #[test]
    fn read_single_quoted_value_with_special_chars() {
        let contents = "HELLO='v a l # \n val\"=val'\n\n".to_string();
        let test_map = process_dot_env(contents).expect("error processing env file");
        assert_eq!(test_map.get("HELLO").unwrap(), "v a l # \n val\"=val")
    }

    /// simple parse and serialize fully
    #[test]
    fn parse_and_serialize() {
        let contents = fs::read_to_string("tests/Test.env").expect("unable to read file");
        let env_test_map = process_dot_env(contents).expect("unable to process env");
        serialize_new_env("tests/TestSerialize.env".to_string(), env_test_map)
            .expect("unable to serialize env");
    }
}
