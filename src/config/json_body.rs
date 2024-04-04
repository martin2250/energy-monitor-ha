use picoserve::{extract::FromRequest, response::StatusCode};
use serde::Deserialize;

pub struct JsonBody<T>(pub T);

impl<'r, State, T> FromRequest<'r, State> for JsonBody<T>
where
    T: Deserialize<'r>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request<R: picoserve::io::Read>(
        _state: &'r State,
        _request_parts: picoserve::request::RequestParts<'r>,
        request_body: picoserve::request::RequestBody<'r, R>,
    ) -> Result<Self, Self::Rejection> {
        let Ok(result) = request_body.read_all().await else {
            return Err((StatusCode::BAD_REQUEST, "error reading request"));
        };
        let Ok((value, _)) = serde_json_core::from_slice(result) else {
            return Err((StatusCode::BAD_REQUEST, "error decoding json"));
        };
        Ok(JsonBody(value))
    }
}
