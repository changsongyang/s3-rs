use http::{HeaderMap, HeaderValue, StatusCode};

use crate::error::{Error, Result};

pub(crate) const MAX_LIST_OBJECTS_KEYS: u32 = 1_000;
#[cfg(feature = "multipart")]
pub(crate) const MAX_LIST_PARTS: u32 = 1_000;

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

pub(crate) fn validate_max_keys(max_keys: u32) -> Result<()> {
    if max_keys == 0 || max_keys > MAX_LIST_OBJECTS_KEYS {
        return Err(Error::invalid_config(
            "max_keys must be in the range 1..=1000",
        ));
    }
    Ok(())
}

#[cfg(feature = "multipart")]
pub(crate) fn validate_max_parts(max_parts: u32) -> Result<()> {
    if max_parts == 0 || max_parts > MAX_LIST_PARTS {
        return Err(Error::invalid_config(
            "max_parts must be in the range 1..=1000",
        ));
    }
    Ok(())
}

pub(crate) fn validate_subresource(subresource: &str) -> Result<()> {
    if subresource.trim().is_empty() {
        return Err(Error::invalid_config("subresource must not be empty"));
    }
    Ok(())
}

pub(crate) fn apply_metadata_headers(
    headers: &mut HeaderMap,
    metadata: Vec<(String, String)>,
) -> Result<()> {
    for (name, value) in metadata {
        let header_name = crate::util::redact::metadata_header_name(&name)?;
        let value = HeaderValue::from_str(&value)
            .map_err(|_| Error::invalid_config("invalid metadata header value"))?;
        headers.insert(header_name, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

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

    #[test]
    fn validate_max_keys_accepts_range_and_rejects_out_of_range() {
        assert!(validate_max_keys(1).is_ok());
        assert!(validate_max_keys(1_000).is_ok());
        assert!(validate_max_keys(0).is_err());
        assert!(validate_max_keys(1_001).is_err());
    }

    #[cfg(feature = "multipart")]
    #[test]
    fn validate_max_parts_accepts_range_and_rejects_out_of_range() {
        assert!(validate_max_parts(1).is_ok());
        assert!(validate_max_parts(1_000).is_ok());
        assert!(validate_max_parts(0).is_err());
        assert!(validate_max_parts(1_001).is_err());
    }

    #[test]
    fn validate_subresource_rejects_blank_values() {
        assert!(validate_subresource("versioning").is_ok());
        assert!(validate_subresource("").is_err());
        assert!(validate_subresource("   ").is_err());
    }

    #[test]
    fn apply_metadata_headers_writes_expected_headers() {
        let mut headers = HeaderMap::new();
        apply_metadata_headers(
            &mut headers,
            vec![
                ("owner".to_string(), "alice".to_string()),
                ("trace-id".to_string(), "abc-123".to_string()),
            ],
        )
        .expect("metadata should map to headers");

        assert_eq!(
            headers
                .get("x-amz-meta-owner")
                .and_then(|v| v.to_str().ok()),
            Some("alice")
        );
        assert_eq!(
            headers
                .get("x-amz-meta-trace-id")
                .and_then(|v| v.to_str().ok()),
            Some("abc-123")
        );
    }

    #[test]
    fn parse_xml_or_service_error_maps_request_id_only_error_payload() {
        let body = "<Error><RequestId>req-only</RequestId></Error>";
        let err = parse_xml_or_service_error::<()>(
            http::StatusCode::BAD_REQUEST,
            &http::HeaderMap::new(),
            body,
            |_| Err(Error::decode("failed to parse expected xml", None)),
        )
        .expect_err("request-id-only payload should map to API error");

        match err {
            Error::Api {
                status, request_id, ..
            } => {
                assert_eq!(status, http::StatusCode::BAD_REQUEST);
                assert_eq!(request_id.as_deref(), Some("req-only"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }
}
