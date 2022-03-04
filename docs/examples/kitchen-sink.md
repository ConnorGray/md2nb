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

#### Text

Hard breaks are supported. \
This is a separate line without a paragraph break.
This is in the same paragraph, without a hard break.

#### Links

This is an [inline](https://example.org) link.

This is a [full reference][full reference] link.

This is a [shortcut] reference link.

This is an autolink: <https://example.org>.

[full reference]: https://example.org
[shortcut]: https://example.org

#### Code blocks

```rust
fn it_also_has_code_block() {
    println!("hello world!");
}
```

Indented code blocks are supported:

    "This is an indented code block."

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

Block quotes support hard breaks:

> First line. \
> Second line.

Block quotes support styled text:

> Block quote with *italicized **and** bolded* text, nested.

##### Nested block quotes

> Block quotes can be nested
>
> > This is useful for representing conversations in markdown.
> >
> > > The block quotes can be nested to an arbitrary depth.

In addition to containing nested block quotes, block quotes can also contain code blocks:

> Block quotes can be nested
>
> ```wolfram
> Print["This is some quoted code!"]
> ```
>
> ```python
> print("This is some quoted code!")
> ```

#### Tables

| Column A | Column B | Third Column |
|----------|----------|--------------|
| Foo      | Fizz     | ✅           |
| Baz      | Buzz     | ❌           |
| This is a row with some longer content, that might even word wrap. | Content in separate columns will word wrap separately. Text in tables can *also be italicized* or **bolded**. | ❔ |

#### Horizontal rules

Horizontal rules can be used to visually split the document:

***

This is after the rule.
