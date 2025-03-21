use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Frequency {
    hours: i64,
    start_time: Option<i64>,
}

impl Frequency {
    pub fn new(hours: i64) -> Frequency {
        Frequency {
            hours,
            start_time: None,
        }
    }

    pub fn get_hours(&self) -> i64 {
        return self.hours;
    }

    // every 6 hours
    // 4 times a day
    pub fn parse(frequency: &str) -> Option<Self> {
        let lower = frequency.to_lowercase();
        let mut split = lower.split(" ");
        match split.next() {
            Some(token) if token == "every" => {
                let first_token = split.next();
                let second_token = split.next();

                match first_token {
                    Some(token) => {
                        if let Ok(number) = token.parse::<i64>() {
                            // every 5
                            match second_token {
                                Some(token)
                                    if token == "hour" || token == "hours" || token == "h" =>
                                {
                                    Some(Frequency {
                                        hours: number,
                                        start_time: None,
                                    })
                                }
                                Some(token) if token == "day" || token == "days" => {
                                    Some(Frequency {
                                        hours: number * 24,
                                        start_time: None,
                                    })
                                }
                                Some(_) => None,
                                None => None,
                            }
                        } else if token == "day" {
                            Some(Frequency {
                                hours: 24,
                                start_time: None,
                            })
                        } else if token == "hour" {
                            Some(Frequency {
                                hours: 1,
                                start_time: None,
                            })
                        } else if token.ends_with("h") {
                            let len = token.len();
                            match &token[..len - 1].to_string().parse::<i64>() {
                                Err(_e) => None,
                                Ok(value) => Some(Frequency {
                                    hours: *value,
                                    start_time: None,
                                }),
                            }
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
            Some(token) if token.parse::<usize>().is_ok() => {
                let number = token.parse::<usize>().unwrap();
                let rest_tokens: Vec<&str> = split.collect();

                if rest_tokens.first() == Some(&"times") && rest_tokens.last() == Some(&"day") {
                    return Some(Frequency {
                        hours: (24f64 / number as f64).floor() as i64,
                        start_time: None,
                    });
                }

                None
            }
            Some(_) => None,
            None => None,
        }
    }
}

impl Display for Frequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "every {} hours", self.hours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frequency_every() {
        assert_eq!(
            Frequency::parse("every 6 hours"),
            Some(Frequency {
                hours: 6,
                start_time: None
            })
        );
    }

    #[test]
    fn test_parse_frequency_every_day() {
        assert_eq!(
            Frequency::parse("every day"),
            Some(Frequency {
                hours: 24,
                start_time: None
            })
        );
    }

    #[test]
    fn test_parse_frequency_times() {
        assert_eq!(
            Frequency::parse("3 times a day"),
            Some(Frequency {
                hours: 8,
                start_time: None
            })
        );
    }

    #[test]
    fn test_parse_frequency_malformed() {
        assert_eq!(Frequency::parse("lol no way"), None);
    }
}
