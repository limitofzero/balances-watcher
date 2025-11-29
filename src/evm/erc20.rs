use alloy::{sol};

sol! {
   #[sol(rpc)]
   contract ERC20 {
        function balanceOf(address owner) public view returns (uint256);
   }
}