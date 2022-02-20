use pulldown_cmark::{self as md, Event, Tag};

//======================================
// Representation
//======================================

#[derive(Debug)]
pub(crate) enum UnflattenedEvent<'a> {
    /// This [`Event`] can never by [`Event::Start`] or [`Event::End`]. Those events
    /// are represented by
    Event(Event<'a>),
    Nested {
        tag: Tag<'a>,
        events: Vec<UnflattenedEvent<'a>>,
    },
}

//======================================
// Implementation
//======================================

pub(crate) fn parse_markdown_to_unflattened_events(input: &str) -> Vec<UnflattenedEvent> {
    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = md::Options::empty();
    options.insert(md::Options::ENABLE_STRIKETHROUGH);
    options.insert(md::Options::ENABLE_TABLES);
    let parser = md::Parser::new_ext(input, options);

    let mut unflattener = Unflattener {
        root: vec![],
        nested: vec![],
    };

    for event in parser {
        unflattener.handle_event(event);
    }

    unflattener.finish()
}

struct Unflattener<'a> {
    root: Vec<UnflattenedEvent<'a>>,
    nested: Vec<(Tag<'a>, Vec<UnflattenedEvent<'a>>)>,
}

impl<'a> Unflattener<'a> {
    fn handle_event(&mut self, event: Event<'a>) {
        match event {
            Event::Start(tag) => {
                self.nested.push((tag, vec![]));
            },
            Event::End(tag) => {
                let (tag2, inner) = self.nested.pop().expect("expected nested events");

                debug_assert_eq!(tag, tag2);

                self.seq()
                    .push(UnflattenedEvent::Nested { tag, events: inner });
            },
            event => self.seq().push(UnflattenedEvent::Event(event)),
        }
    }

    fn seq(&mut self) -> &mut Vec<UnflattenedEvent<'a>> {
        if let Some((_, seq)) = self.nested.last_mut() {
            seq
        } else {
            &mut self.root
        }
    }

    fn finish(self) -> Vec<UnflattenedEvent<'a>> {
        let Unflattener { root, nested } = self;

        assert!(nested.is_empty());

        root
    }
}
