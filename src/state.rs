use std::ops::Deref;

use crate::request::Request;

#[derive(Debug, Default, Clone, Copy)]
pub struct State<S>(pub S);

pub trait FromRequest<S> {
    fn from_request(state: State<S>, request: Request) -> Self;
}

impl<S> FromRequest<S> for S
where
    S: Clone,
{
    fn from_request(state: State<S>, _request: Request) -> Self {
        let State(inner_state) = state;
        inner_state
    }
}

impl<S> Deref for State<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
