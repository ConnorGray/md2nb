# md2nb

`md2nb` is a command-line tool for converting [Markdown](https://wikipedia.org/wiki/Markdown)
files into [Wolfram Notebooks](https://wolfram.com/notebooks).

![Diagram showing md2nb conversion of Markdown files to Wolfram Notebooks](./docs/images/md2nb.png)

## Features

* Convert `.md` files into Wolfram `.nb` files.
* Most markdown constructs can be converted into cannonical Wolfram Notebook
  representations.

* [ ] Embeds the content of image links into the notebook

## Usage

`md2nb` is a command-line tool. After [installing `md2nb`](#installation), it can be used
to convert a `.md` file to a `.nb` like so:

```shell
$ md2nb <INPUT>.md <OUTPUT>.nb
```

For example, to convert this project's `README.md` file into a Wolfram Notebook, execute:

```shell
$ md2nb README.md README.nb
```

## Installation

*TODO*
