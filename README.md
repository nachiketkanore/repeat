### Repeat: Repeats a command until a condition is met.

```
Usage: repeat [OPTIONS] <COMMAND>...

Arguments:
  <COMMAND>...  The command and its arguments to execute repeatedly. Example: repeat --exit-code 127 echo "Running..."

Options:
      --exit-code <CODE>         Exit the repeating loop when the command returns this specific exit code
      --run-timeout-sec <SECS>   Maximum time (in seconds) allowed for a single execution. Kills the process if exceeded
  -v, --verbose                  Enable verbose logging of each run's output, exit code, and duration
  -i, --iterations <ITERATIONS>  Number of iterations for a given command [default: 10]
  -h, --help                     Print help
  -V, --version                  Print version
```
