export function formatTimestamp(timestamp, showDate = true) {
  if (timestamp == null) return ''

  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) return ''

  const pad = (value, length = 2) => String(value).padStart(length, '0')
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`

  if (!showDate) return time

  return `${pad(date.getFullYear(), 4)}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${time}`
}
