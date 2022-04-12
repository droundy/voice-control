# Voice control

This crate (still in its infancy) is intended to allow you to control your
computer with your voice.  I want it to work for programming the computer.

Currently it doesn't really work in a useful way, but has some of the structure
it will need.  It can listen to your default microphone, interpret what you say,
and control your computer.  But it doesn't understand very well at all.  I
believe I need to train a language model.  There is a program
`examples/log-keystrokes` that can log all your keystrokes to generate a corpus
for training the model.  Obviously, you'll want to be careful in using this not
to type anything sensitive.

I also need to refine the grammar, which is currently quite simple.