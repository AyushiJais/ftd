# ftd


## Basic

```
-- amitu/table: Some table
columns: l | c | r 

some body
```

```json
{
    "section": "amitu/table",
    "caption": "Some table",
    "columns": "l | c | r",
    "body": "some body"
}
```

- caption is optional, if not passed it should be set to empty string.
- keys can contain alphanumeric, `-` and `_`.

## With Nested Object

```
-- amitu/table: Some table
columns: l | c | r 

some body

--- something: something caption
s_key: yo

something body
```

```json
{
    "section": "amitu/table",
    "caption": "Some table",
    "columns": "l | c | r",
    "body": "some body",
    "something": {
        "caption": "something caption",
        "s_key": "yo",
        "body": "something body"
    }
}
```

## With Arrays:

```
-- amitu/table: Some table
columns: l | c | r 

some body

--- something: something caption
s_key: yo

something body

--- row[]: row 1
r: row1's r

row 1's body

-- row[]: row 2
r: row1's r

row 2's body
```

```json
{
    "section": "amitu/table",
    "caption": "Some table",
    "columns": "l | c | r",
    "body": "some body",
    "something": {
        "caption": "something caption",
        "s_key": "yo",
        "body": "something body"
    },
    "rows[]": [
        { 
            "caption": "row 1",
            "r": "row1's r",
            "body": "row 1's body"
        },
        { 
            "caption": "row 2",
            "r": "row2's r",
            "body": "row 2's body"
        }
    ]
}
```

## On Body

- Body should have at least single empty line before and after it.
- Empty lines before and after the body would be removed.

```
-- amitu/table: Some table

some body



```

```json
{
    "section": "amitu/table",
    "caption": "Some table",
    "body": "some body"
}
```

## Escaping in Body

If any line body is supposed to start with either `-- ` or `--- `, they would be escaped with "\": `\-- ` and `\--- ` respectively:

```
-- amitu/table: Some table

\-- yo

this is cool

\--- something

yo yo yo


```

```json
{
    "section": "amitu/table",
    "caption": "Some table",
    "body": "-- yo\n\nthis is cool\n\n--- something\n\nyo yo yo"
}
```

