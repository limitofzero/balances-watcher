use alloy::sol;

sol! {
    // ABI Multicall3 @ 0xcA11bde05977b3631167028862bE2a173976CA11
    // Verified source shows these structs & functions.
    #[sol(rpc)]
    contract Multicall3 {
        struct Call {
            address target;
            bytes callData;
        }

        struct Call3 {
            address target;
            bool allowFailure;
            bytes callData;
        }

        struct Call3Value {
            address target;
            bool allowFailure;
            uint256 value;
            bytes callData;
        }

        struct Result {
            bool success;
            bytes returnData;
        }

        // ---------- aggregate family ----------
        function aggregate(Call[] calls)
            public
            payable
            returns (uint256 blockNumber, bytes[] returnData);

        function tryAggregate(bool requireSuccess, Call[] calls)
            public
            payable
            returns (Result[] returnData);

        function tryBlockAndAggregate(bool requireSuccess, Call[] calls)
            public
            payable
            returns (uint256 blockNumber, bytes32 blockHash, Result[] returnData);

        function blockAndAggregate(Call[] calls)
            public
            payable
            returns (uint256 blockNumber, bytes32 blockHash, Result[] returnData);

        function aggregate3(Call3[] calls)
            public
            payable
            returns (Result[] returnData);

        function aggregate3Value(Call3Value[] calls)
            public
            payable
            returns (Result[] returnData);

        function getBlockHash(uint256 blockNumber) public view returns (bytes32 blockHash);
        function getBlockNumber() public view returns (uint256 blockNumber);
        function getCurrentBlockCoinbase() public view returns (address coinbase);
        function getCurrentBlockDifficulty() public view returns (uint256 difficulty);
        function getCurrentBlockGasLimit() public view returns (uint256 gaslimit);
        function getCurrentBlockTimestamp() public view returns (uint256 timestamp);

        function getEthBalance(address addr) public view returns (uint256 balance);

        function getLastBlockHash() public view returns (bytes32 blockHash);
        function getBasefee() public view returns (uint256 basefee);
        function getChainId() public view returns (uint256 chainid);
    }
}
