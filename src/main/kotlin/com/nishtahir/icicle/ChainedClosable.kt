package com.nishtahir.icicle

import java.io.Closeable

class ChainedCloseable<T : Closeable>(private val payload: T, private val parents: List<Closeable>) {
    fun <U> use(block: (T) -> U): U {
        try {
            return block(payload)
        } finally {
            payload.close()
            parents.asReversed().forEach { it.close() }
        }
    }

    fun <U : Closeable> useWith(block: (T) -> U): ChainedCloseable<U> {
        val newPayload = block(payload)
        return ChainedCloseable(newPayload, parents + payload)
    }
}

fun <T : Closeable, U : Closeable> T.useWith(block: (T) -> U): ChainedCloseable<U> {
    val newPayload = block(this)
    return ChainedCloseable(newPayload, listOf(this, newPayload))
}
