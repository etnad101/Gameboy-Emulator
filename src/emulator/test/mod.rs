use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct State {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
    pub ram: Vec<Vec<u16>>,
}

#[derive(Deserialize, Debug)]
pub struct TestData {
    pub name: String,
    pub initial: State,
    #[serde(rename = "final")]
    pub final_name: State,
}
