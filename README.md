# Squads multi CPI example

The idea of this repo is to have a working example with all the CPI calls needed to create and execute a Squads multisig transaction.

The final point is for the program (stakePoolPDA) to be one of the signers to mint a SPLToken, so when a user deposit N-SOL in the stakePool, the stakePool creates and execute the mint transaction via Squads.


### CURRENT STATUS
[AccountBorrowFailed] Failed to borrow a reference to account data, already borrowed. (At trying to approve the created proposal)
