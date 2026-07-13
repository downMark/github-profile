import { request } from './client'
import type { GithubUser } from '../types/user'

export interface UserListResult {
  items: GithubUser[]
  total: number
  page: number
  limit: number
}

/** POST /api/users — 用 GitHub Token 导入用户 */
export function importUser(token: string): Promise<GithubUser> {
  return request<GithubUser>('/users', {
    method: 'POST',
    body: JSON.stringify({ token }),
  })
}

/** GET /api/users — 分页用户列表（按 updated_at DESC，响应不含 encrypted_token） */
export async function listUsers(page = 1, limit = 20): Promise<UserListResult> {
  const query = new URLSearchParams({ page: String(page), limit: String(limit) })
  return request<UserListResult>(`/users?${query.toString()}`)
}

/** GET /api/users/:id — 单用户详情 */
export function getUser(id: string): Promise<GithubUser> {
  return request<GithubUser>(`/users/${encodeURIComponent(id)}`)
}

/** POST /api/users/:id/refresh — 用存储 token 刷新用户资料 */
export function refreshUser(id: string): Promise<GithubUser> {
  return request<GithubUser>(`/users/${encodeURIComponent(id)}/refresh`, {
    method: 'POST',
  })
}
