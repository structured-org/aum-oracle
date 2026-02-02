// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {Receiver} from "../src/Receiver.sol";

contract ReceiverTest is Test {
    Receiver rec;
    address alice = address(0xA11CE);
    address bob = address(0xB0B);

    function setUp() public {
        vm.prank(alice);
        rec = new Receiver(alice);
    }

    function testPublish() public {
        vm.prank(alice);
        rec.publish(123, 456);
        (uint256 er, uint256 ts) = rec.getLatest();
        assertEq(er, 123);
        assertEq(ts, 456);
    }

    function testOnlyPublisher() public {
        vm.expectRevert(Receiver.NotPublisher.selector);
        vm.prank(bob);
        rec.publish(1, 2);
    }

    function testSetPublisher() public {
        vm.startPrank(alice);
        rec.setPublisher(bob);
        vm.stopPrank();

        vm.prank(bob);
        rec.publish(7, 9);
        (uint256 er, uint256 ts) = rec.getLatest();
        assertEq(er, 7);
        assertEq(ts, 9);
    }
}
