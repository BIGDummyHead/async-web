use chrono::{DateTime, Duration, Local};

/// The current state of the API, this usually will include items like:
///
/// * ok
/// * exhausted
///
/// And other API states that may occur from under/over usage of the API.
#[derive(Debug)]
pub enum ApiState {
    Ok,
    Exhausted,
    //add other states here if applicable
}

impl std::fmt::Display for ApiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        let err = match self {
            ApiState::Ok => "",
            ApiState::Exhausted => "api calls have been exhausted, please try again later"
        }.to_string();

        write!(f, "{err}")
    }
}

impl std::error::Error for ApiState {}

/// Struct that tracks specific meta of an API call.
pub struct ApiMeta {
    //time of the call
    pub time: DateTime<Local>,
    //add other information here if needed.
}

impl ApiMeta {

    pub fn new() -> Self {
        Self { time: Local::now() }
    }

    ///checks if the given api meta reference has expired.
    fn expired(&self, max_allowed_duration: &Duration) -> bool {
        let current_time = Local::now();

        let dif = current_time - self.time;

        dif >= *max_allowed_duration
    }
}

pub struct ApiHandler {
    //the amount of calls currently to this endpoint.
    max_meta_calls: usize,
    //the calls currently not purged (valid calls)
    meta_calls: Vec<ApiMeta>,
    /// the amount of time between a api meta and when it gets purged.
    /// for example:
    ///
    /// If the time is 10:00am (local) and we had two incoming calls at 7:59am and 8:01am.
    ///
    /// If we also have the max_allowed_duration set to 2 hours and call the purge function.
    ///
    /// 7:59 will be removed from the meta_calls, effectively meaning that there was only one valid call.
    max_allowed_duration: Duration,
}

impl ApiHandler {
    pub fn new(max_calls: usize, max_duration: std::time::Duration) -> ApiHandler {
        Self {
            meta_calls: vec![],
            max_allowed_duration: chrono::Duration::from_std( max_duration ).expect("invalid time provided"),
            max_meta_calls: max_calls,
        }
    }

    /// # can_request
    ///
    /// Determines if the API call can be made  
    pub fn can_request(&mut self) -> bool {
        //retain only non_expired meta calls.
        self.retain_non_expired();

        self.meta_calls.len() < self.max_meta_calls
    }

    //gets the current state of the API.
    pub fn get_state(&mut self) -> ApiState {
        if self.can_request() {
            ApiState::Ok
        } else {
            ApiState::Exhausted
        }
    }

    /// Checks and attemts to make the call to the given API.
    ///
    /// If the call is successful, the result returns ok.
    ///
    /// If the call is not, the ApiState is returned back.
    pub fn make_call(&mut self) -> Result<(), ApiState> {
        match self.get_state() {
            ApiState::Exhausted => return Err(ApiState::Exhausted),
            _ => {},
        };

        let meta = ApiMeta::new();

        self.meta_calls.push(meta);
        
        Ok(())
    }

    //retains any request that are not expired.
    fn retain_non_expired(&mut self) {
        self.meta_calls
            .retain(|meta| !meta.expired(&self.max_allowed_duration));
    }
}
