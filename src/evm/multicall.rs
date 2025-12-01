use alloy::sol;

sol! {
    struct Call {
        address target;
        bytes callData;
    }

    struct Result {
        bool success;
        bytes returnData;
    }

    #[sol(rpc)]
    contract Multicall {
        function tryAggregate(bool requireSuccess, Call[] calldata calls) public view returns (Result[] returnData);
    }
}