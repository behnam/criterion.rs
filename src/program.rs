use std::fmt;
use std::io::BufReader;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use DurationExt;
use routine::Routine;

// A two-way channel to the standard streams of a child process
pub struct Program {
    buffer: String,
    stdin: ChildStdin,
    // NB Don't move the `stdin` field, because it must be dropped first
    _child: Child,
    stderr: ChildStderr,
    stdout: BufReader<ChildStdout>,
}

impl Program {
    pub fn spawn(cmd: &mut Command) -> Program {
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());

        let mut child = match cmd.spawn() {
            Err(e) => panic!("`{:?}`: {}", cmd, e),
            Ok(child) => child,
        };

        Program {
            buffer: String::new(),
            stderr: child.stderr.take().unwrap(),
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            _child: child,
        }
    }

    pub fn send<T>(&mut self, line: T) -> &mut Program where
        T: fmt::Display,
    {
        use std::io::Write;

        match writeln!(&mut self.stdin, "{}", line) {
            Err(e) => panic!("`write into child stdin`: {}", e),
            Ok(_) => self,
        }
    }

    pub fn recv(&mut self) -> &str {
        use std::io::{BufRead, Read};

        self.buffer.clear();

        match self.stdout.read_line(&mut self.buffer) {
            Err(e) => {
                self.buffer.clear();

                match self.stderr.read_to_string(&mut self.buffer) {
                    Err(e) => {
                        panic!("`read from child stderr`: {}", e);
                    },
                    Ok(_) => {
                        println!("stderr:\n{}", self.buffer);
                    }
                }

                panic!("`read from child stdout`: {}", e);
            },
            Ok(_) => &self.buffer,
        }
    }
}

impl Routine for Program {
    fn bench<I>(&mut self, iters: I) -> Vec<f64> where I: Iterator<Item=u64> {
        let mut n = 0;
        for iters in iters {
            self.send(iters);
            n += 1;
        }

        (0..n).map(|_| {
            let msg = self.recv();
            let msg = msg.trim();

            let elapsed: u64 = msg.parse().expect("Couldn't parse program output");
            elapsed as f64
        }).collect()
    }

    fn warm_up(&mut self, how_long_ns: Duration) -> (u64, u64) {
        let mut iters = 1;

        let mut total_iters = 0;
        let start = Instant::now();
        loop {
            self.send(iters).recv();

            total_iters += iters;
            let elapsed = start.elapsed();
            if elapsed > how_long_ns {
                return (elapsed.to_nanos(), total_iters);
            }

            iters *= 2;
        }
    }
}
