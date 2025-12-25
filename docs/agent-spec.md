# Sapling Agent Specification

In Sapling the lowest layer of data is called a subject and is either a string, number, boolean or unique identifier. The next and last layer is called a fact. A fact is a tuple of the following subjects:
- Target subject
  - Used to attach certain information to a single subject
- Property subject
  - Used to define certain information of a certain property
- Value subject
  - Used to define the value of a certain property at a given target subject
- Operator subject
  - Used to define the operator of a certain property at a given target subject

## Syntax

```
TARGET_SUBJECT PROPERTY_SUBJECT OPERATOR_SUBJECT VALUE_SUBJECT
```

## Core principles

The only form of computation in sapling is the evaluation of a subject which is either express as `targetSubject` or `?targetSubject` where the first literally yields all facts where matches `targetSubject * *`, and the latter executes the subject in the "evaluation mode".

### Evaluation mode

Evaluation mode is the core of Sapling and is a form of unification. The first step is retrieve all fact requirements in non-evaluation mode, and then yields the unfication result where `_TARGET_SUBJECT reqProperty1 = reqProperty1ExpectedSubject` & `_TARGET_SUBJECT reqProperty2 = reqProperty2ExpectedSubject` and so on. Evaluation can be "nested" so to say by setting the expected subject's evaluation mode on with `?expectedSubjectX`

#### Unification

Unification is the process of finding a substitution that makes two terms equal. In Sapling, unification is used to match facts and evaluate expressions. If the same subject in evaluation mode is used multiple times, the unification will be done across the whole evaluation.

## Examples
```
country_de country = 'germany'
country_de currency = 'euro'
country_pl country = 'poland'
country_pl currency = 'zloty'
country_uk country = 'uk'
country_uk currency = 'pound'
country_fr country = 'france'
country_fr currency = 'euro'
country_xx currency = 'euro'
countries_with_euro currency == 'euro'

# Output for ?countries_with_euro
# country_de currency = 'euro'
# country_fr currency = 'euro'
# country_xx currency = 'euro'

countries_with_euro_and_name currency == 'euro'
countries_with_euro_and_name country == *

# Output for ?countries_with_euro_and_name
# country_de country = 'germany'
# country_de currency = 'euro'
# country_fr country = 'france'
# country_fr currency = 'euro'

person1 name = 'Alice'
person1 age = 30
person1 country = country_de
person2 name = 'Bob'
person2 age = 25
person2 country = country_uk
person3 name = 'Charlie'
person3 age = 35
person3 country = country_fr
person4 name = 'Diana'
person4 age = 28
person4 country = country_pl
person5 name = 'Eve'
person5 age = 22
person5 country = country_fr
person6 name = 'Frank'
person6 age = 40
person6 country = country_de
person7 name = 'Grace'
person7 age = 27
person7 country = country_pl

euro_countries currency == 'euro'

people_in_euro_countries name == *
people_in_euro_countries age == *
people_in_euro_countries country == ?euro_countries

# Output for ?people_in_euro_countries
# person1 name = 'Alice'
# person1 age = 30
# person1 country = country_de
# person3 name = 'Charlie'
# person3 age = 35
# person3 country = country_fr
# person5 name = 'Eve'
# person5 age = 22
# person5 country = country_fr
# person6 name = 'Frank'
# person6 age = 40
# person6 country = country_de

data1 a = 1
data1 b = 2
data2 a = 1
data2 b = 4
data3 a = 2
data3 b = 2
data4 a = 4
data4 b = 2
data5 a = 1
data5 b = 1
data6 a = 6
data6 b = 4

someA a == ?value

# Output for someA
# someA a == ?value

# Output for ?someA
# data1 a = 1
# data2 a = 1
# data3 a = 2
# data4 a = 4
# data5 a = 1
# data6 a = 6

sameAandB a == ?value
sameAandB b == ?value

# Output for sameAandB
# sameAandB a == ?value
# sameAandB b == ?value

# Output for ?sameAandB
# data3 a = 2
# data3 b = 2
# data5 a = 1
# data5 b = 1
```
