use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub address: String,
    pub name: String,
    pub decimals: u8,
    pub chain_id: u16,
}