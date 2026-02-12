use http::{HeaderMap, StatusCode};

use crate::error::Result;

pub(crate) fn parse_xml_or_service_error<T>(
    status: StatusCode,
    headers: &HeaderMap,
    body: &str,
    parse: impl FnOnce(&str) -> Result<T>,
) -> Result<T> {
    match parse(body) {
        Ok(value) => Ok(value),
        Err(parse_error) => {
            if crate::util::xml::parse_error_xml(body).is_some() {
                return Err(crate::transport::response_error_from_status(
                    status, headers, body,
                ));
            }
            Err(parse_error)
        }
    }
}

pub(crate) fn create_bucket_location_constraint(
    explicit: Option<String>,
    client_region: &str,
) -> Option<String> {
    match explicit {
        Some(region) => Some(region),
        None if client_region.eq_ignore_ascii_case("us-east-1") => None,
        None => Some(client_region.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bucket_location_constraint_defaults_to_client_region() {
        assert_eq!(
            create_bucket_location_constraint(None, "ap-southeast-1"),
            Some("ap-southeast-1".to_string())
        );
    }

    #[test]
    fn create_bucket_location_constraint_skips_us_east_1_by_default() {
        assert_eq!(create_bucket_location_constraint(None, "us-east-1"), None);
        assert_eq!(create_bucket_location_constraint(None, "US-EAST-1"), None);
    }

    #[test]
    fn create_bucket_location_constraint_respects_explicit_value() {
        assert_eq!(
            create_bucket_location_constraint(Some("eu-west-1".to_string()), "us-east-1"),
            Some("eu-west-1".to_string())
        );
    }
}
