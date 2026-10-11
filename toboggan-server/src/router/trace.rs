use axum::http::{Response, StatusCode, header};
use tower_http::classify::{
    ClassifiedResponse, ClassifyResponse, NeverClassifyEos, ServerErrorsAsFailures,
    ServerErrorsFailureClass, SharedClassifier,
};
use tower_http::trace::TraceLayer;

/// The request log every router wraps itself in.
pub(super) fn trace_layer() -> TraceLayer<SharedClassifier<RetryLaterIsNotFailure>> {
    TraceLayer::new(SharedClassifier::new(RetryLaterIsNotFailure))
}

/// [`ServerErrorsAsFailures`], except for a `503` that carries `Retry-After`.
///
/// That response is an answer, not a fault: `/overview/slide/{index}` sends it
/// while the deck is still being photographed, and the client asks again when
/// told to. Counted as a failure, `TraceLayer` logged it at `ERROR`, once per
/// probe, on every start — so the one log line that should make a speaker look
/// up was the one they learned to ignore. A `503` with no `Retry-After` (the
/// PDF route when `typst` fails) is still a failure.
#[derive(Debug, Clone, Copy)]
pub(super) struct RetryLaterIsNotFailure;

impl ClassifyResponse for RetryLaterIsNotFailure {
    type FailureClass = ServerErrorsFailureClass;
    type ClassifyEos = NeverClassifyEos<ServerErrorsFailureClass>;

    fn classify_response<B>(
        self,
        res: &Response<B>,
    ) -> ClassifiedResponse<Self::FailureClass, Self::ClassifyEos> {
        if res.status() == StatusCode::SERVICE_UNAVAILABLE
            && res.headers().contains_key(header::RETRY_AFTER)
        {
            return ClassifiedResponse::Ready(Ok(()));
        }
        ServerErrorsAsFailures::new().classify_response(res)
    }

    fn classify_error<E>(self, error: &E) -> Self::FailureClass
    where
        E: std::fmt::Display + 'static,
    {
        ServerErrorsAsFailures::new().classify_error(error)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn is_failure(status: StatusCode, retry_after: bool) -> bool {
        let mut builder = Response::builder().status(status);
        if retry_after {
            builder = builder.header(header::RETRY_AFTER, "1");
        }
        let response = builder.body(()).expect("valid response");
        matches!(
            RetryLaterIsNotFailure.classify_response(&response),
            ClassifiedResponse::Ready(Err(_))
        )
    }

    #[test]
    fn a_503_that_says_when_to_retry_is_not_a_failure() {
        assert!(!is_failure(StatusCode::SERVICE_UNAVAILABLE, true));
    }

    #[test]
    fn a_503_that_does_not_is_still_a_failure() {
        assert!(is_failure(StatusCode::SERVICE_UNAVAILABLE, false));
    }

    #[test]
    fn retry_after_excuses_no_other_server_error() {
        assert!(is_failure(StatusCode::INTERNAL_SERVER_ERROR, true));
    }

    #[test]
    fn success_and_client_errors_are_not_failures() {
        assert!(!is_failure(StatusCode::OK, false));
        assert!(!is_failure(StatusCode::NOT_FOUND, false));
    }
}
