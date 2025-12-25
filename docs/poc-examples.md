
## Intro
```sh
rene/name = "Rene"
    /lastName = "Eichhorn"

max/name = "Max"
   /lastName = "Musterman"

tim/name = "Tim"
   /lastName = "Miten"


allPeople/name == *
         /lastName == *


> allPeople
>> rene/..., max/..., tim/...

> tim
>> tim/name = ..
>> tim/lastName = ..
```


## UI Example
```sh
myForm/ui::element = ui::form
      /children = pwLabel/ui::element = ui::label
                         /children = "Enter Password"
      /children = pwInput/ui::element = ui::input
      /children = pwLogin/ui::element = ui::button
                         /children = "Login"


loginButton/ui::element == ui::button
            /children == "Login"
> loginButton
>> pwLogin/ui::element = ..., /children = ...

loginButton/ui::element == ui::button
           /children ui::says "Login"
> loginButton
>> pwLogin/ui::element = ..., /children = ...

```

## Operators
```sh
foo/name == "Foo"
foo/name system::equals "Foo"
# vs
foo/name = "Foo"
foo/name system::assign "Foo"

queryQuery/name ?== "Foo"
> queryQuery
>> query/name = "Foo"
```


## Sub databases
```sh
myContainer/ui::element = ui::container
           /state       = 1
           /state_+1    = system::add/0 = myContainer/state
                                     /1 = 1
                                     /value
           /children    = decr /ui::element = ui::button
                               /children = "Decrement"
                          label/ui::element = ui::label
                               /children = myContainer/state
                          incr /ui::element = ui::button
                               /children = "Increment"
                               /ui::click = [myContainer/state = myContainer/state_+1 #SNAPSHOT ]



foo/onMessage = [_/new = 1]

queryDbs/system::db::subject == myContainer
        /system::db::target == *
                    /state == *

queryDbs/system::db::subject == myContainer
        /system::db::target == incr/ui::click
                    /state == *
```

## Virtual value vs executed value

```sh
foo/a = 1
a = sum/0 = foo/a
       /1 = 1
       /value
> a
>> a/value = 2

foo/a = 2
> a
>> a/value = 3

b = sum/0 = foo/a
       /1 = 1
       /value #NOW

> b
>> b/value = 3

foo/a = 100

> b
>> b/value = 3

```

## Most recent vs array

```sh

a/0 = 1
a/0 = 2
a/0 = 3

> a
>> a/0 = 3
> a #ALL
>> a/0 = 1, a/0 = 1, ...

query/0 == * #ALL
> query
>> a/0 = 1, a/0 = 1, ...
```

## Metadata
```sh
foo/name = "Rene" #PUBLIC

#MY_CUSTOM_META/system::meta::type = system::meta::type::access
               /system::meta::user = system::root

#MY_OTHER_META/...

foo/name = "Rene" #MY_CUSTOM_META #MY_OTHER_META
```


## "Reusables" / "Functions"


```sh
x_plus_x/output = system::add/0 = x_plus_x/value
                             /1 = x_plus_x/value
                             /output #NEW SCOPE

5_plus_5 = x_plus_x/value = 5
                   /output #NEW SCOPE

single_add = system::add/0 = 1
                        /output #NEW SCOPE

> single_add
>> _/single_add = 1 NOT 6
```

## Array vs Single

```sh
foo/0 = 1
foo/0 = 1
foo/0 = 1
system::add/0 = foo/0
           /1 = 1
           /value
> system::add/value
>> 2

system::add/0 = foo/0 #ALL // TODO: Does the Meta apply to the value or the whole assignment of add/0 = .. or foo/0 itself, if so how to differentiate?
           /1 = 1
           /value
> system::add/value
>> 2, 2, 2

system::add/0 = foo/0 #ALL
           /1 = foo/0 #ALL
           /value
> system::add/value
>> 2, 2, 2

bar/0 = 3
bar/0 = 3
system::add/0 = foo/0 #ALL
           /1 = bar/0 #ALL
           /value
> system::add/value
>> 4, 4
>> TODO: OR MAYBE ERROR?
```

## Array like operations, add, remove, find

```sh
person/friend = max
      /friend = nana
      /friend = rene

addFriendButton/onClick = [
  person/friend = newFriend
]

removeFriendButton/onClick = [
  person/friend /= newFriend
]
```

## Array equals vs one of equals

```sh
personA/friend = max
       /friend = nana

personB/friend = max
       /friend = rene

friendsWithMaxAndRene/friend == rene
                     /friend == max
```

## Dynamic fields

```sh

?allSubjects/foo = "foo field"

someSubjects/firstName == *
            /lastName == *

?someSubjects/name = system::concat/0 = ?someSubjects/firstName
                                   /1 = ?someSubjects/lastName

```

## Unfication variables

```sh
person/name == "Max"
      /parent == ?mother/gender == "female"


person/age == ?age
      /likes == ?likedPerson/age == ?age

sameFirstLastNamePeople/firstName == ?name
                       /lastName == ?name

```


## AND vs OR query conditions

```sh
personReneAged20/name == "Rene"
      /age == 20

peopleCalledReneOrAged20 = system::anyOf/0 = personA/name = "Rene"
                                        /1 = personB/age = 20
```

## whole subject vs last property subject

```sh
foo/bar = baz/a = 0
             /b = 0

>> foo/bar is set to subject baz

foo/bar = baz/a = 0
             /b = 0
             /b

>> foo/bar is set to property b of subject baz
```

## mapping data to other data

```sh
>> source
foo/name = "Foo"
   /age = 18
foo/name = "Foo2"
   /age = 20
>> target
bar/foo = "Foo"
   /age = 19
bar/foo = "Foo2"
   /age = 21

bar/foo = ?foo/name
   /age = system::sum/0 = ?foo/age
                     /1 = 1
```

## reactive mapping for system internals

```sh
foo/name = "Foo"
allFoos/name == *

system::log/message = "Something created: $name"
           /name = ?allFoos/name #ALL

>> LOG: Something created: Foo

bar/name  = "Bar"

>> LOG: Something created: Bar
```

## template string
```sh
system::format/template = "Hello $name!"
              /name = "World"
```

## formular
```sh
system::formular/template = "1 + (12 - $foo) / $bar"
                /foo = 5
                /bar = 12
```

## triggers
```sh
system::trigger::truthy/condition = true
                       /limit = 1
                       /db = [foo/bar = baz]
```

## flappy bird

```sh
bird/position = {0, 0}
    /tickVelocity = {1, -0.3}
    /jumpVelocity = {0, 0.5}


>> Physics tick
system::threads::game/tick = [
    bird/position = system::sum/0 = bird/position
                               /1 = bird/tickVelocity
                               /value #SNAPSHOT
]

system::input::keyPress/space = [
    bird/position = system::sum/0 = bird/position
                               /1 = bird/jumpVelocity
                               /value #SNAPSHOT
]

>> Pipe creation
system::trigger::timer/time = "4s"
                      /trigger = [
                        pipeTop/entity = pipe
                               /position = system::sum/0 = bird/position #SNAPSHOT
                                                      /1 = {5, -10}

                        pipeBottom/entity = pipe
                                  /position = system::sum/0 = bird/position #SNAPSHOT
                                                      /1 = {5, 10}
                      ]

allPipes/entity == pipe
        /position == *

>> Loss condition
lossCondition = system::math::collision/quad = birdQuad/position = bird/position
                                                       /size = 5
                                       /quad = pipes/position = ?allPipes/position
                                                    /size = {10, 1}

system::triggers::onRetract/query = ?lossCondition
                           /db = [system::process/running = false]
```

## Conway's Game of Life

TODO

## Validation

```sh
name/system::property::type = system::property::type::string
    /system::property::minLength = 3
    /system::property::maxLength = 20
    /system::property::regex = "^[A-Za-z]+$"

age/system::property::type = system::property::type::integer
   /system::property::min = 0
   /system::property::max = 120

rene/name = "Rene"
rene/age = 40
max/name = 123 >> ERROR: Invalid type
```

## Error handling

```sh
foo/num = system::sum/0 = 1
                     /1 = "string"
                     /value

> foo/num
>> Yields nothing since no valid data can be computed
> system::sum/1
>> Yields nothing as well
> system::sum/0
>> Yields nothing as well since the entire subject is considered invalid
> foo/num #FORCE
>> foo/num = system::sum....

queryError/num system::error::equals *
> queryError
>> queryError/num = validationError/type = system::error::typemismatch
                                    /message = "system::sum/1 expected a number but got a string"
                                    /sourcePath = root/subject = system::sum #FORCE
                                                      /property = 1 #FORCE
                                                      /value = system::sum/1 #FORCE
                                                  level1/subject = foo #FORCE
                                                        /property = num #FORCE
                                                        /value = foo/num #FORCE
```

## I/O Operations

```sh
myFileContent = system::io::file/path = "data.txt"
                                /mode = "read"
                                /content

myFileContentSafe = system::error::default/source = myFileContent
                                   /default = "Default Content"
                                   /value
```

## Advanced user permission handling

#NDA/system::meta::type = system::meta::type::access
    /system::meta::user = person1
    /system::meta::user = person2
    /system::meta::denial = system::meta::denial::hidden

secretData = "Top Secret Information" #NDA

> secretData
>> nothing for anyone not person1 or person2
>> no error just hidden

### NOTES

SUBJECT/PROPERTY_SUBJECT OPERATOR_SUBJECT VALUE_SUBJECT (#META_DESCRIPTORS, ...)

#### Operators

- `=`, `system::equal`, `equal`: Equality check, literal equality or subject equality
- `==`, `system::assign`, `assign`: Assignment
- `?...`: Reverse search operator, ?= for examples matches with `=` operator, `==` is in an alias for `?=`
- `ui::says`: Matches with anything that would render or print to something fuzzily matching the value
- `system::error::equals`: Similar to `==` but matches with the error on the data if available

#### Value Types
- `subject` other subject
- `subject/property` other subject's property
- `subject/property #META` other subject's property, with meta applied
- constant values "string", 123, 4.56, true, false, ...
- [subject/property operator value, ...] a database

#### Special subjects
- `system::log` upon creation logs to stdout
- `#NOW` upon creations converts to current time metadata
- `#NEW SCOPE` binds data to a new scope this is the default
- `#SNAPSHOT` binds to the timestamp when the statement is executed for example on a click
