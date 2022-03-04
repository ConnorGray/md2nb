# Development

This file contains documentation for developers of this project. Users of this
project do not need to read this file.

### Updating `kitchen-sink.png`

The 'Kitchen Sink' example is intended to demonstrate all Markdown features supported by
`md2nb`. When new samples are added to [`kitchen-sink.md`](./examples/kitchen-sink.md),
the [`kitchen-sink.png`](./images/kitchen-sink.png) file needs to be regenerated.

**Regenerate `kithen-sink.png`:**

1.  Build `kitchen-sink.md` into `kitchen-sink.nb`

    ```shell
    $ cargo run -- ./docs/examples/kitchen-sink.md .
    ```


3.  Open a new notebook, separate from `kitchen-sink.nb`, and run the following code.

    This will rasterize `kitchen-sink.nb` and prompt you for the location to save the
    resulting image as a PNG. Save the file as `md2nb/docs/images/kitchen-sink.png`.

    ```wolfram
    kitchenSinkNB = SelectFirst[
        Notebooks[],
        Information[#, "WindowTitle"] === "kitchen-sink.nb" &
    ];

    image = Rasterize[kitchenSinkNB];

    Export[
        SystemDialogInput["FileSave", "kitchen-sink.png"],
        image,
        "PNG"
    ]
    ```