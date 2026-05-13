#[allow(dead_code)]
pub struct ServiceRequest {
    pub id: u64,
    pub payload: Vec<u8>,
}

#[allow(dead_code)]
pub struct ServiceResponse {
    pub id: u64,
    pub result: Vec<u8>,
}
