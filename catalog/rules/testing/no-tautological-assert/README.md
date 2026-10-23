## Directive

An assertion compares the result of the code under test with a value that does not come from that
result. The rule reports an assertion that holds whatever the code returns. An error by default.

| Language | Reported |
|---|---|
| Python | `assert True`, `assert x == x`, `assert x is x`, `self.assertTrue(True)`, `self.assertEqual(x, x)`, `assertIs`, `assertDictEqual` and `assertListEqual` with the same two arguments |
| TypeScript, TSX, JavaScript | `expect(x).toBe(x)`, `toEqual(x)` and `toStrictEqual(x)` on the same value, `expect(true).toBeTruthy()`, `assert.equal`, `strictEqual`, `deepEqual` and `deepStrictEqual` with the same two arguments, `assert(true)`, `assert.ok(true)` |
| Go | `assert.Equal`, `EqualValues`, `Exactly` and `Same` of testify, and the same of `require`, with the same two values; `assert.True(t, true)` and `require.True(t, true)` |
| Rust | `assert_eq!(x, x)` and `debug_assert_eq!(x, x)`, with or without a message |

Two arguments are the same when their syntax trees are the same.

## Why

A test whose assertion always holds passes when the code under test is broken, and it raises the
coverage of the lines it runs. Ruff reports a tuple or a string asserted
(`F631`, `PLW0129`) and, with the section of ruff of `base-python`, `x == x` and `1 == 1`
(`PLR0124`, `PLR0133`); clippy reports `assert!(true)` (`assertions_on_constants`). `assert True`,
`self.assertEqual(x, x)`, `assert_eq!(x, x)` in a test and the forms of TypeScript and Go reach no
linter of the stack. A Python repository with both sees `assert x == x` twice, once from ruff and
once from botica.

## Exceptions

`assert x == x` that checks that a float is not NaN is reported: write `assert not math.isnan(x)`.
`assert False` and `assert!(false)` mark a branch that must not run and are not reported.

## Do

```python
def test_total():
    assert total([1, 2]) == 3
```

```ts
expect(total([1, 2])).toBe(3);
```

## Don't

```python
def test_total():
    result = total([1, 2])
    assert result == result
```

```go
assert.Equal(t, total, total)
```

## Suppression

`# botica: ignore[no-tautological-assert]` on the line in Python, `// botica: ignore[no-tautological-assert]`
in the other languages, with the reason.
