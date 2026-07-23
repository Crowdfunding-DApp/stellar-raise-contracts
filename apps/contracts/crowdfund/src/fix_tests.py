import os

files_to_fix = [
    "withdraw_event_emission_test.rs",
    "refund_single_token_tests.rs",
    "cargo_toml_rust.test.rs",
    "stellar_token_minter.test.rs",
    "admin_upgrade_mechanism.test.rs",
    "crowdfund_initialize_function.test.rs",
    "refund_single_token_security_tests.rs",
    "admin_upgrade_mechanism_test.rs",
    "contract_state_size.test.rs",
    "refund_single_token_test.rs",
    "auth_tests.rs",
    "test.rs",
    "crowdfund_initialize_function_test.rs",
    "stellar_token_minter_test.rs",
    "contribute_error_handling_tests.rs",
    "refund_single_token.test.rs"
]

for filename in files_to_fix:
    path = os.path.join("/home/gamp/grantfox/stellar-raise-contracts/apps/contracts/crowdfund/src", filename)
    if not os.path.exists(path):
        continue
    with open(path, "r") as f:
        content = f.read()
    
    # We need to replace `&None,\n    );` with `&None,\n        &7,\n    );`
    # Also `&None\n    );`
    import re
    content = re.sub(r'(&None,)\s*\n\s*\);', r'\1\n        &7,\n    );', content)
    content = re.sub(r'(&None)\s*\n\s*\);', r'\1,\n        &7,\n    );', content)
    # also try_initialize
    with open(path, "w") as f:
        f.write(content)

print("Done")
