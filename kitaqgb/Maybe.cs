using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;

// Provide the untyped Nothing sentinel and a type-inferred non-null Just factory.
public struct Maybe
{
    public static readonly Maybe Nothing = new Maybe();

    // Reject null before constructing a populated optional value.
    public static Maybe<T> Just<T>(T value)
    {
        if (value == null)
        {
            throw new ArgumentNullException("A Maybe value may not contain null.");
        }
        return Maybe<T>.Just(value);
    }
}

[DebuggerDisplay("{DebuggerDisplay,nq}")]
// Store zero or one value and expose it as an enumerable. Default construction is empty.
public struct Maybe<T> : IEquatable<Maybe<T>>, IEnumerable<T>
{
    public bool HasValue { get; private set; }
    private T InternalValue;

    // Return the stored value only when populated; reading an empty optional throws.
    public T Value
    {
        [DebuggerStepThrough]
        get
        {
            if (!HasValue)
            {
                throw new InvalidOperationException("A value cannot be retrieved from a Maybe object constructed with 'Nothing'.");
            }
            return InternalValue;
        }
    }

    // Store the supplied flag/value directly. Unlike Just, this public constructor does not reject a populated null.
    public Maybe(bool hasValue, T value)
    {
        HasValue = hasValue;
        InternalValue = value;
    }

    // Use the ordinary diagnostic representation in debugger variable views.
    private string DebuggerDisplay
    {
        get
        {
            return ToString();
        }
    }

    // Construct a populated optional after rejecting null references.
    public static Maybe<T> Just(T value)
    {
        if (value == null)
        {
            throw new ArgumentNullException("A Maybe value may not contain null.");
        }
        return new Maybe<T>(true, value);
    }

    public static readonly Maybe<T> Nothing = new Maybe<T>(false, default(T));

    // Promote a non-null value through Just, preserving its null rejection.
    public static implicit operator Maybe<T>(T value)
    {
        return Just(value);
    }

    // Convert the untyped sentinel to this element type's empty optional.
    public static implicit operator Maybe<T>(Maybe value)
    {
        return Nothing;
    }

    // Treat any Maybe like a "TryGetValue" call. Return true if a value was present.
    public bool TryGet(out T value)
    {
        if (HasValue)
        {
            value = InternalValue;
            return true;
        }
        else
        {
            value = default(T);
            return false;
        }
    }

    // Given Just(x), return x.
    // Given Nothing, return defaultValue.
    public T Or(T defaultValue)
    {
        return HasValue ? InternalValue : defaultValue;
    }

    // Given Just(x), return x.
    // Given Nothing, return alternative.
    public Maybe<T> Or(Maybe<T> alternative)
    {
        return HasValue ? this : alternative;
    }

    // Given Nothing, return Nothing.
    // Given Just(x), return Just(function(x)).
    // Invoke the selector only when populated; a null selector result is rejected by Just.
    public Maybe<TResult> Select<TResult>(Func<T, TResult> function)
    {
        return HasValue ? Maybe.Just(function(InternalValue)) : Maybe.Nothing;
    }

    // Return all the elements that satisfy the predicate.
    // Keep a populated value only when its predicate succeeds. TResult is unused by this signature.
    public Maybe<T> Where<TResult>(Func<T, bool> predicate)
    {
        return (HasValue && predicate(InternalValue)) ? this : Maybe.Nothing;
    }

    // Given Just(x), perform action(x).
    // Given Nothing, do nothing.
    public void Act(Action<T> action)
    {
        if (HasValue)
        {
            action(Value);
        }
    }

    // Render Nothing or the contained value's text; a populated null bypassing Just is not supported here.
    public override string ToString()
    {
        return HasValue
            ? ("Just " + InternalValue.ToString())
            : "Nothing";
    }

    // Use zero for empty and delegate to the populated value's hash code.
    public override int GetHashCode()
    {
        return HasValue ? InternalValue.GetHashCode() : 0;
    }

    // Accept only an optional with the same element type, then compare its state/value.
    public override bool Equals(object obj)
    {
        if (obj is Maybe<T>)
        {
            return Equals((Maybe<T>)obj);
        }
        else
        {
            return false;
        }
    }

    // Empty values compare equal; populated values use the contained type's equality.
    public bool Equals(Maybe<T> other)
    {
        if (!HasValue && !other.HasValue)
        {
            // Both values are Nothing.
            return true;
        }
        else if (HasValue && other.HasValue)
        {
            // Both values are something; test equality.
            return InternalValue.Equals(other.InternalValue);
        }
        else
        {
            // One value is something and the other is Nothing.
            return false;
        }
    }

    // Delegate equality to the optional state/value comparison.
    public static bool operator ==(Maybe<T> left, Maybe<T> right)
    {
        return left.Equals(right);
    }

    // Negate optional state/value equality.
    public static bool operator !=(Maybe<T> left, Maybe<T> right)
    {
        return !left.Equals(right);
    }

    // Lazily enumerate the stored value once when populated, otherwise yield no elements.
    IEnumerator<T> IEnumerable<T>.GetEnumerator()
    {
        if (HasValue) yield return InternalValue;
    }

    // Provide the same zero-or-one sequence through the non-generic enumeration interface.
    IEnumerator IEnumerable.GetEnumerator()
    {
        if (HasValue) yield return InternalValue;
    }
}

public static class MaybeExtensions
{
    // Return the value corresponding to the specified key.
    // Return Nothing if the key is not in the dictionary.
    // Return a found dictionary value through the implicit Just conversion; stored null values are rejected.
    public static Maybe<V> Get<K, V>(this IDictionary<K, V> dictionary, K key)
    {
        V value;
        if (dictionary.TryGetValue(key, out value))
        {
            return value;
        }
        else
        {
            return Maybe.Nothing;
        }
    }

    // Return the value at the specified index.
    // Return Nothing if the index is out of bounds.
    // Return Nothing for an out-of-range index; an in-range null value is rejected by implicit conversion.
    public static Maybe<T> Get<T>(this IList<T> list, int index)
    {
        if (index >= 0 && index < list.Count)
        {
            return list[index];
        }
        else
        {
            return Maybe.Nothing;
        }
    }

    // Lazily filter out empty optionals while preserving the order of populated values.
    public static IEnumerable<T> Values<T>(this IEnumerable<Maybe<T>> source)
    {
        foreach (Maybe<T> maybe in source)
        {
            if (maybe.HasValue)
            {
                yield return maybe.Value;
            }
        }
    }

    // Apply a function to each item in a sequence. Return only the non-Nothing results.
    public static IEnumerable<R> SelectMaybe<T, R>(this IEnumerable<T> source, Func<T, Maybe<R>> selector)
    {
        foreach (T item in source)
        {
            Maybe<R> result = selector(item);
            if (result.HasValue)
            {
                yield return result.Value;
            }
        }
    }
}




