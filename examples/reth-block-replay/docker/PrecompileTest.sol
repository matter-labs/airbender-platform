// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract PrecompileTest {
    event EcrecoverResult(address recovered);
    event Sha256Result(bytes32 digest);
    event Ripemd160Result(bytes20 digest);
    event Bn256AddResult(bytes32 x, bytes32 y);
    event Bn256MulResult(bytes32 x, bytes32 y);

    function callAll(
        bytes32 msgHash,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external {
        address recovered = ecrecover(msgHash, v, r, s);
        require(recovered != address(0), "ecrecover failed");
        emit EcrecoverResult(recovered);
        emit Sha256Result(sha256(abi.encodePacked(msgHash, recovered)));
        emit Ripemd160Result(ripemd160(abi.encodePacked(msgHash, recovered)));

        bytes memory addInput = abi.encodePacked(
            uint256(1), uint256(2),
            uint256(1), uint256(2)
        );
        (bool addOk, bytes memory addOut) = address(0x06).staticcall(addInput);
        require(addOk && addOut.length == 64, "bn256Add failed");
        emit Bn256AddResult(bytes32(slice(addOut, 0)), bytes32(slice(addOut, 32)));

        bytes memory mulInput = abi.encodePacked(
            uint256(1), uint256(2),
            uint256(7)
        );
        (bool mulOk, bytes memory mulOut) = address(0x07).staticcall(mulInput);
        require(mulOk && mulOut.length == 64, "bn256ScalarMul failed");
        emit Bn256MulResult(bytes32(slice(mulOut, 0)), bytes32(slice(mulOut, 32)));
    }

    function slice(bytes memory data, uint256 start) internal pure returns (bytes32 result) {
        assembly {
            result := mload(add(add(data, 32), start))
        }
    }
}
