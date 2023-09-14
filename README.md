# bbspack

A fork of Pangaea's original ( [super-continent/ggst-bbs-unpacker](https://github.com/super-continent/ggst-bbs-unpacker) ), built specifically to cater to my personal workflow.

## Changes from vanilla ggst-bbs-unpacker

- Yeeted out GUI

- Minor code cleanup

## Usage

```
bbspack extract BBS_CMNEF.uexp BBS_CMNEF.dbzscript
bbspack inject BBS_CMNEF.dbzscript BBS_CMNEF.uexp BBS_CMNEF.uasset
```

I use the file extension 'dbzscript' to indicate unparsed BBS, but you can use anything you want. File extensions don't really matter.

Note that you still need to parse the output with [dobosken/bbscript](https://github.com/dobosken/bbscript) or [super-continent/bbscript](https://github.com/super-continent/bbscript) to get nice, readable script. Don't forget to rebuild your script before injecting it back in though!
