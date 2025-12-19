use alloy::{sol};

sol! {
   #[sol(rpc)]
   contract ERC20 {
        function balanceOf(address owner) public view returns (uint256);

        #[derive(Debug)]
        event Transfer(address indexed from, address indexed to, uint256 value);
   }
}