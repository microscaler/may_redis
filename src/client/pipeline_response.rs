// Pipeline response type conversion for may-redis
//
// Provides the `FromPipelineResponse` trait and implementations for
// extracting typed results from pipeline responses.

use crate::core::{FromRedisValue, RedisError, RedisValue};

/// Trait for extracting typed results from multiple pipeline responses.
///
/// Implemented for single tuples `(T1,)`, pairs `(T1, T2)`, triples
/// `(T1, T2, T3)`, and `Vec<T>` to cover the most common pipeline use cases.
pub trait FromPipelineResponse: Sized {
    /// Convert a vector of `RedisValue` responses into `Self`.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the number of responses does not match
    /// the expected count for the target type, or if a response cannot be
    /// converted to the requested Rust type.
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError>;
}

// ---------------------------------------------------------------------------
// FromPipelineResponse implementations
// ---------------------------------------------------------------------------

impl<T1: FromRedisValue> FromPipelineResponse for (T1,) {
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 1 {
            return Err(RedisError::Parse(format!(
                "expected 1 response, got {}",
                responses.len()
            )));
        }
        let r0 = responses[0].clone();
        let t1 = T1::from_redis_value(&r0)?;
        Ok((t1,))
    }
}

impl<T1: FromRedisValue, T2: FromRedisValue> FromPipelineResponse for (T1, T2) {
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 2 {
            return Err(RedisError::Parse(format!(
                "expected 2 responses, got {}",
                responses.len()
            )));
        }
        let r0 = responses[0].clone();
        let r1 = responses[1].clone();
        let t1 = T1::from_redis_value(&r0)?;
        let t2 = T2::from_redis_value(&r1)?;
        Ok((t1, t2))
    }
}

impl<T1: FromRedisValue, T2: FromRedisValue, T3: FromRedisValue> FromPipelineResponse
    for (T1, T2, T3)
{
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 3 {
            return Err(RedisError::Parse(format!(
                "expected 3 responses, got {}",
                responses.len()
            )));
        }
        let r0 = responses[0].clone();
        let r1 = responses[1].clone();
        let r2 = responses[2].clone();
        let t1 = T1::from_redis_value(&r0)?;
        let t2 = T2::from_redis_value(&r1)?;
        let t3 = T3::from_redis_value(&r2)?;
        Ok((t1, t2, t3))
    }
}

impl<T1: FromRedisValue, T2: FromRedisValue, T3: FromRedisValue, T4: FromRedisValue>
    FromPipelineResponse for (T1, T2, T3, T4)
{
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 4 {
            return Err(RedisError::Parse(format!(
                "expected 4 responses, got {}",
                responses.len()
            )));
        }
        let r0 = responses[0].clone();
        let r1 = responses[1].clone();
        let r2 = responses[2].clone();
        let r3 = responses[3].clone();
        let t1 = T1::from_redis_value(&r0)?;
        let t2 = T2::from_redis_value(&r1)?;
        let t3 = T3::from_redis_value(&r2)?;
        let t4 = T4::from_redis_value(&r3)?;
        Ok((t1, t2, t3, t4))
    }
}

impl<T: FromRedisValue> FromPipelineResponse for Vec<T> {
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        let mut result = Self::with_capacity(responses.len());
        for response in responses {
            result.push(T::from_redis_value(&response)?);
        }
        Ok(result)
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_pipeline_response_single() {
        let responses = vec![RedisValue::Integer(42)];
        let result: Result<(i64,), _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(result.unwrap(), (42,));
    }

    #[test]
    fn test_from_pipeline_response_pair() {
        let responses = vec![
            RedisValue::Integer(1),
            RedisValue::BulkString(b"hello".to_vec()),
        ];
        let result: Result<(bool, String), _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(result.unwrap(), (true, "hello".to_string()));
    }

    #[test]
    fn test_from_pipeline_response_triple() {
        let responses = vec![
            RedisValue::Integer(1),
            RedisValue::Integer(2),
            RedisValue::Integer(3),
        ];
        let result: Result<(bool, i64, i64), _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(result.unwrap(), (true, 2, 3));
    }

    #[test]
    fn test_from_pipeline_response_vec() {
        let responses = vec![
            RedisValue::BulkString(b"a".to_vec()),
            RedisValue::BulkString(b"b".to_vec()),
            RedisValue::BulkString(b"c".to_vec()),
        ];
        let result: Result<Vec<String>, _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(
            result.unwrap(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_from_pipeline_response_wrong_count() {
        let responses = vec![RedisValue::Integer(1)];
        let result: Result<(i64, i64), _> = FromPipelineResponse::from_responses(responses);
        assert!(result.is_err());
    }
}
