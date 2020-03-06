# Substrate rsrl (Machine learning) (POW)
Example of using rsrl together with blockchain to run machine learning on Simple Boxcar problem.
During the machine learning process, the association rules are built and are referred as "Policy" in the rsrl. The initial difficulty is 100. This difficulty number means the number of "Episodes" to run during the training. This difficulty will increase over time. The computation time will increase as the difficulty increases, hence fufilling the POW requirement. In peer setup, the incoming "Policy" will be saved inside the blockchain as verified block only if the "Policy" is better. To determine if the "Policy" is better, a test evaluation will be run. In this simple Boxcar problem, the lower number of steps to complete the problem, the better "Policy" it is. When subsequent peers connect to the blockchain, they should start performing machine learning from the best "Policy".

# TO Do
- Change Difficulty to usize
- Run multiple evaluations and calculate the mean steps in the Compute function instead of single evaluation 

# Instruction
cd node-template
git submodule update --init --recursive
make sure the vendor/substrate hash is db1ab7d18fbe7876cdea43bbf30f147ddd263f94, checkout to this hash if necessary

cargo run -- --dev --validator