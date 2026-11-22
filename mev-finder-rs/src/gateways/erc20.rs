// Общий ABI ERC-20, не зависящий от Uniswap v4.

use alloy::sol;

sol! {
    #[sol(rpc)]
    interface ERC20 {
        function balanceOf(address owner) external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);
        function approve(address spender, uint256 amount) external returns (bool);
    }
}
