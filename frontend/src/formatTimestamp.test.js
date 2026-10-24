import assert from 'node:assert/strict'
import test from 'node:test'
import { formatTimestamp } from './formatTimestamp.js'

test('includes the local date and three-digit milliseconds by default', () => {
  const timestamp = new Date(2026, 0, 2, 3, 4, 5, 6).getTime()
  assert.equal(formatTimestamp(timestamp), '2026-01-02 03:04:05.006')
})

test('hides only the date when disabled', () => {
  const timestamp = new Date(2026, 8, 30, 13, 14, 15, 123).getTime()
  assert.equal(formatTimestamp(timestamp, false), '13:14:15.123')
})

test('keeps zero and two-digit milliseconds padded', () => {
  assert.equal(formatTimestamp(new Date(2026, 0, 1, 0, 0, 0, 0).getTime()), '2026-01-01 00:00:00.000')
  assert.equal(formatTimestamp(new Date(2026, 0, 1, 0, 0, 0, 45).getTime(), false), '00:00:00.045')
})

test('handles a year boundary without rounding milliseconds', () => {
  const timestamp = new Date(2026, 11, 31, 23, 59, 59, 999).getTime()
  assert.equal(formatTimestamp(timestamp), '2026-12-31 23:59:59.999')
  assert.equal(formatTimestamp(timestamp + 1), '2027-01-01 00:00:00.000')
})

test('accepts the Unix epoch and leaves missing or invalid values blank', () => {
  assert.match(formatTimestamp(0), /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.000$/)
  for (const timestamp of [undefined, null, NaN, Infinity, 'invalid']) {
    assert.equal(formatTimestamp(timestamp), '')
  }
})
