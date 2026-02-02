// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

contract Receiver {
    address public publisher;
    uint256 private er;
    uint256 private ts;

    event PublisherUpdated(
        address indexed oldPublisher,
        address indexed newPublisher
    );

    event ValuesPublished(uint256 er, uint256 ts);

    error NotPublisher();

    constructor(address _publisher) {
        require(_publisher != address(0), "zero publisher");
        publisher = _publisher;
    }

    modifier onlyPublisher() {
        if (msg.sender != publisher) revert NotPublisher();
        _;
    }

    function setPublisher(address newPublisher) external onlyPublisher {
        require(newPublisher != address(0), "zero addr");
        emit PublisherUpdated(publisher, newPublisher);
        publisher = newPublisher;
    }

    function publish(uint256 newEr, uint256 newTs) external onlyPublisher {
        er = newEr;
        ts = newTs;
        emit ValuesPublished(newEr, newTs);
    }

    function getLatest() external view returns (uint256 _er, uint256 _ts) {
        return (er, ts);
    }
}
