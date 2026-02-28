use alloy::sol;

sol! {
   #[sol(rpc)]
   contract WrappedToken {
    // when user wrap/unwrap token - it emits Deposit/Withdrawal event (not transfer as for erc20)
    #[derive(Debug)]
    event Deposit(address indexed dst, uint256 wad);

    #[derive(Debug)]
    event Withdrawal(address indexed src, uint256 wad);
   }
}
