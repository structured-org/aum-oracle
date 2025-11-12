// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {Receiver} from "../src/Receiver.sol";

contract ReceiverDeploy is Script {
    function run() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address publisher = vm.envAddress("PUBLISHER");

        vm.startBroadcast(pk);

        Receiver receiver = new Receiver(publisher);

        vm.stopBroadcast();

        console2.log("Receiver deployed at:", address(receiver));
    }
}
