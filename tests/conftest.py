import os

# Ensure all pytest executions run with ACTX_TEST_MODE="1" by default
# to prevent automated tests from polluting or triggering production cascade purges
os.environ["ACTX_TEST_MODE"] = "1"
