#[derive(Debug, PartialEq)]
pub struct Frequency {
    hours: usize,
    start_time: Option<usize>,
}

impl Frequency {
    pub fn new(hours: usize) -> Frequency {
        Frequency {
            hours,
            start_time: None,
        }
    }

    // every 6 hours
    // 4 times a day
    pub fn parse(frequency: &str) -> Option<Self> {
        let mut split = frequency.split(" ");
        match split.next() {
            Some(token) if token == "every" => {
                let first_token = split.next();
                let second_token = split.next();

                match first_token {
                    Some(token) => {
                        if let Ok(number) = token.parse::<usize>() {
                            // every 5
                            match second_token {
                                Some(token) if token == "hour" || token == "hours" => Some(Frequency {
                                    hours: number,
                                    start_time: None,
                                }),
                                Some(token) if token == "day" || token == "days" => Some(Frequency {
                                    hours: number * 24,
                                    start_time: None,
                                }),
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
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
            Some(token) if matches!(token.parse::<usize>(), Ok(_)) => {
                let number = token.parse::<usize>().unwrap();

                let rest_tokens: Vec<&str> = split.collect();

                let last_token = rest_tokens.last().unwrap();

                if *rest_tokens.first().unwrap() == "times" {
                    if last_token.matches("day").count() > 0 {
                        return Some(Frequency {
                            hours: (24f64 / number as f64).floor() as usize,
                            start_time: None,
                        });
                    }
                }

                None
            }
            Some(_) => None,
            None => None,
        }
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
