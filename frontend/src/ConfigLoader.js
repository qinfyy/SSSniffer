import axios from 'axios'

async function loadConfig(path) {
  try {
    const response = await axios.get(path, { responseType: 'text' })
    const config = parseIni(response.data)
    return config || {}

  } catch (error) {
    if (error.response && error.response.status === 404) {
      console.warn('Config.ini not found, returning empty object')
    } else {
      console.warn('Failed to load Config.ini, returning empty object:', error)
    }
    return {}
  }
}

function parseIni(text) {
  const result = {}
  let currentSection = null
  
  const lines = text.split('\n')
  
  for (const line of lines) {
    const trimmedLine = line.trim()

    if (!trimmedLine || trimmedLine.startsWith(';') || trimmedLine.startsWith('#')) {
      continue
    }

    const sectionMatch = trimmedLine.match(/^\[(.+)\]$/)
    if (sectionMatch) {
      currentSection = sectionMatch[1]
      result[currentSection] = {}
      continue
    }

    const keyValueMatch = trimmedLine.match(/^([^=]+)=(.*)$/)
    if (keyValueMatch && currentSection) {
      const key = keyValueMatch[1].trim()
      const value = keyValueMatch[2].trim()
      result[currentSection][key] = value
    }
  }
  
  return result
}

export {
  loadConfig,
};
