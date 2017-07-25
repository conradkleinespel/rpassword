#!/usr/bin/env bash

set -e

DIR="$( dirname $( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd ) )"

# STDIN comes from a terminal: we can't properly unit test this, so we need to
# test by hand. The following test must be made:
# - ./target/debug/examples/tests > tests/stdout 2> tests/stderr
# - press 1, ENTER, 2, ENTER, 3, ENTER, 4, ENTER, 5, ENTER, 6, ENTER
# - you should not see 1/2/3 displayed, but you should see 4/5/6 displayed
# - the following diffs should be empty
#   - diff tests/stdout <(printf "1\nprompt_password_stdout2\n3\n4\nprompt_reply_stdout5\n6\n")
#   - diff tests/stderr <(printf "prompt_password_stderrprompt_reply_stderr")

# STDIN comes from a pipe
printf "1\n2\n3\n4\n5\n6\n" | $DIR/target/debug/examples/tests > $DIR/tests/results_pipe_actual_stdout 2> $DIR/tests/results_pipe_actual_stderr
diff $DIR/tests/results_pipe_expected_stdout $DIR/tests/results_pipe_actual_stdout
echo 'pipe stdout OK'
diff $DIR/tests/results_pipe_expected_stderr $DIR/tests/results_pipe_actual_stderr
echo 'pipe stderr OK'

# missing newline, only tested in pipe mode
printf "1\n2\n3\n4\n5\n6" | $DIR/target/debug/examples/tests > $DIR/tests/results_pipe_missing_nl_actual_stdout 2> $DIR/tests/results_pipe_missing_nl_actual_stderr
diff $DIR/tests/results_pipe_missing_nl_expected_stdout $DIR/tests/results_pipe_missing_nl_actual_stdout
echo 'pipe with missing nl stdout OK'
diff $DIR/tests/results_pipe_missing_nl_expected_stderr $DIR/tests/results_pipe_missing_nl_actual_stderr
echo 'pipe with missing nl stderr OK'
