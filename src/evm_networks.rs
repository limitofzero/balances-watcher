#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub  enum EvmNetworks {
    Eth = 1,
    Arbitrum = 42161,
}

impl EvmNetworks {
    fn chain_id(self) -> u16 {
        self as u16
    }
}

impl TryFrom<u64> for EvmNetworks {
    type Error = ();

    fn try_from(id: u64) -> Result<Self, ()> {
        match id {
            1 => Ok(EvmNetworks::Eth),
            42161 => Ok(EvmNetworks::Arbitrum),
            _ => Err(()),
        }
    }
}
