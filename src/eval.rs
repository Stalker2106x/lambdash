
use std::env;
use std::process;
use std::process::Stdio;

use crate::cmdoutput::CmdOutput;
use crate::core::{ShellError, ShellState};
use crate::command::{self, parse_tokens};
use crate::tokenizer::{Token, tokenize};
use crate::builtins::match_builtin;

#[derive(Debug)]
pub struct ExecutionError {
    pub status: i32,
    pub details: String
}

impl ExecutionError {
    pub fn new(code: i32, msg: String) -> ExecutionError {
        ExecutionError{status: code, details: msg.to_string()}
    }
}

// Expanding

pub fn expand_variable(state: &mut ShellState, var_name: &str) -> String {
    match var_name {
        "?" => format!("{}", state.status),
        _ => {
            match env::var(var_name) {
                Ok(var_value) => var_value,
                Err(_) => format!("${}", var_name)
            }
        }
    }
}

pub fn expand_tokens(state: &mut ShellState, tokens: &mut Vec<Token>) {
    // Iterate over each token in the vector
    for token in tokens.iter_mut() {
        match token {
            Token::Variable(var_name) => {
                *token = Token::Word(expand_variable(state, var_name));
            }
            _ => {}
        }
    }
}

// Execution

pub fn run_command(state: &mut ShellState, command: &Vec<command::Command>) -> Result<CmdOutput, ShellError> {
    let mut output: CmdOutput = CmdOutput::new();
    let mut pipe = false;
    for step in command {
        let cmd = &step.words[0];
        let args = step.words[1..].to_vec();
        let input = if pipe { Some(output.clone()) } else { None };
        match match_builtin(state, cmd, &args, &input) {
            Ok(out) => {
                output.combine(&out);
            }
            Err(error) => {
                match error {
                    ShellError::NoBuiltin => {
                        match execute(cmd, &args, &input) {
                            Ok(out) => {
                                output.combine(&out);
                            },
                            Err(err) => return Err(err)
                        }
                    },
                    error => return Err(error)
                }
            }
        }
        pipe = true;
    }
    return Ok(output);
}

pub fn execute(command: &str, args: &Vec<String>, input: &Option<CmdOutput>) -> Result<CmdOutput, ShellError> {
    let mut process = process::Command::new(command);
    process.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // If no input, execute the command normally
    match process.output() {
        Ok(output) => Ok(CmdOutput::from_output(&output)),
        Err(_error) => Err(ShellError::Execution(ExecutionError::new(
            127,
            format!("{}: command not found", command),
        ))),
    }
}

// Eval

pub fn eval_expr(state: &mut ShellState, expr: &String) -> Result<Option<CmdOutput>, ShellError> {
    match tokenize(expr) {
        Ok(mut tokens) => {
            if tokens.len() > 0 {
                expand_tokens(state, &mut tokens);
                match parse_tokens(&tokens) {
                    Ok(commands) => {
                        let mut output = CmdOutput::new();
                        for cmd in commands {
                            match run_command(state, &cmd) {
                                Ok(out) => output.combine(&out),
                                Err(error) => return Err(error)
                            }
                        }
                        return Ok(Some(output));
                    },
                    Err(error) => return Err(ShellError::Execution(ExecutionError::new(1, format!("invalid syntax"))))
                }
            }
            return Ok(None)
        },
        Err(error) => return Err(ShellError::Tokenization(error))
    };
}