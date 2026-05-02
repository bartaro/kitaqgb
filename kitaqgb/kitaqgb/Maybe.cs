using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;

public struct Maybe
{
    public static readonly Maybe Nothing = new Maybe();

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
public struct Maybe<T> : IEquatable<Maybe<T>>, IEnumerable<T>
{
    public bool HasValue { get; private set; }
    private T InternalValue;

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

    public Maybe(bool hasValue, T value)
    {
        HasValue = hasValue;
        InternalValue = value;
    }

    private string DebuggerDisplay
    {
        get
        {
            return ToString();
        }
    }

    public static Maybe<T> Just(T value)
    {
        if (value == null)
        {
            throw new ArgumentNullException("A Maybe value may not contain null.");
        }
        return new Maybe<T>(true, value);
    }

    public static readonly Maybe<T> Nothing = new Maybe<T>(false, default(T));

    public static implicit operator Maybe<T>(T value)
    {
        return Just(value);
    }

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
    public Maybe<TResult> Select<TResult>(Func<T, TResult> function)
    {
        return HasValue ? Maybe.Just(function(InternalValue)) : Maybe.Nothing;
    }

    // Return all the elements that satisfy the predicate.
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

    public override string ToString()
    {
        return HasValue
            ? ("Just " + InternalValue.ToString())
            : "Nothing";
    }

    public override int GetHashCode()
    {
        return HasValue ? InternalValue.GetHashCode() : 0;
    }

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

    public static bool operator ==(Maybe<T> left, Maybe<T> right)
    {
        return left.Equals(right);
    }

    public static bool operator !=(Maybe<T> left, Maybe<T> right)
    {
        return !left.Equals(right);
    }

    IEnumerator<T> IEnumerable<T>.GetEnumerator()
    {
        if (HasValue) yield return InternalValue;
    }

    IEnumerator IEnumerable.GetEnumerator()
    {
        if (HasValue) yield return InternalValue;
    }
}

public static class MaybeExtensions
{
    // Return the value corresponding to the specified key.
    // Return Nothing if the key is not in the dictionary.
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




