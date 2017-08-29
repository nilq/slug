<img src="http://nilq.dk/slug.png" width="16%" height="16%" align="right">

## slug

it's kinda python, but optionally typed and transpiled to lua

### currently works

```
a num = 10
b num = 10 + a
```

```
fun add (a num, b num) num:
  a + b
  
fun idk (a num): 1
fun hm (a, b): 10
fun ay num: 0
fun c: 0

a = fun num: 10
b = fun: 10
```

```
b any .. = [
  a = fun (b num) num:
    b + 10
]
```

### also works ...

skipping fun call type checking by explicitly declaring the fun as `any` ..

```
a any = fun (b num) num:
    b + 10

c = a "feature hrm"
```
