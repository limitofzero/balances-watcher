use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum EvmNetworks {
    Eth = 1,
    Arbitrum = 42161,
}

impl EvmNetworks {
    fn chain_id(self) -> u16 {
        self as u16
    }
}

impl TryFrom<u64> for EvmNetworks {
    type Error = String;

    fn try_from(id: u64) -> Result<Self, String> {
        match id {
            1 => Ok(EvmNetworks::Eth),
            42161 => Ok(EvmNetworks::Arbitrum),
            other => Err(format!("unknown EVM network id: {}", other)),
        }
    }
}

impl<'de> Deserialize<'de> for EvmNetworks {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let id: u64 = s.parse().map_err(serde::de::Error::custom)?;
        EvmNetworks::try_from(id).map_err(serde::de::Error::custom)
    }
}