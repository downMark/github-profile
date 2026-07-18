import { request } from './client'

export interface Account { id: string; username: string }
export interface AuthResponse {
  access_token: string
  token_type: 'Bearer'
  expires_in: number
  account: Account
}

export function register(username: string, password: string) {
  return request<AuthResponse>('/auth/register', { method: 'POST', body: JSON.stringify({ username, password }) }, false)
}
export function login(username: string, password: string) {
  return request<AuthResponse>('/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) }, false)
}
export function refresh() { return request<AuthResponse>('/auth/refresh', { method: 'POST' }, false) }
export function logout() { return request<void>('/auth/logout', { method: 'POST' }, false) }
