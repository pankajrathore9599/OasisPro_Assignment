Token should have regular balance data stores that CW-20 has.
Token should have a "Frozen Balance" for each account
When tokens are transferred, frozen balance should be checked to make sure that amount is locked and not able to be transferred
Have a balance cap for each token holder (eg. balance cap for each user = 1000, users can only hold up to 1000 tokens.)
Have to do required checks for minting and transferring to make sure balance cap never goes over the cap for any token holders
Lastly, create tests that check that these functions are working properly. 



cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --name my-token
