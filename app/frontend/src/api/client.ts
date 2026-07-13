/** API 基础封装：地址走 VITE_API_BASE_URL，默认 /api（同源反代，见 AGENTS.md 约定） */
const API_BASE_URL: string = import.meta.env.VITE_API_BASE_URL ?? '/api'

export class ApiError extends Error {
  readonly status: number
  readonly code?: string

  constructor(status: number, message: string, code?: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

export async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let response: Response
  try {
    response = await fetch(`${API_BASE_URL}${path}`, {
      headers: { 'Content-Type': 'application/json' },
      ...init,
    })
  } catch {
    throw new ApiError(0, '网络错误，请检查连接后重试')
  }

  if (!response.ok) {
    let message = `请求失败（${response.status}）`
    let code: string | undefined
    try {
      const body: unknown = await response.json()
      if (body && typeof body === 'object') {
        const { message: msg, code: c } = body as { message?: unknown; code?: unknown }
        if (typeof msg === 'string' && msg) message = msg
        if (typeof c === 'string') code = c
      }
    } catch {
      // 响应体不是 JSON，使用默认错误信息
    }
    throw new ApiError(response.status, message, code)
  }

  return (await response.json()) as T
}
