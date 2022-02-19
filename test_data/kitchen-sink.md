# Kitchen Sink Example

This example has some *basic* text.

* It also has common Markdown features, like lists.
* And **bold** text.
  - With nested list items.
    * `md2nb` supports nested lists up to three levels deep.

Links are [also supported](https://example.org).

#### Sub headings can be used to provide structure

##### H5 content

###### H6 content

## Feature Coverage

#### Code blocks

```rust
fn it_also_has_code_block() {
    println!("hello world!");
}
```

##### Conversion of languages supported by `"ExternalLanguage"` cells

```python
for c in "python":
  print(c)
```

```shell
echo $HOME
```

#### Block quotes

> This is a single-line block quote.

This is some content in between.

> This is a multiline block quote.
> It just goes on and on. It will word wrap automatically when viewed in a Wolfram
> Notebook.
>
> Empty lines within the block quote will render as empty lines in the notebook.