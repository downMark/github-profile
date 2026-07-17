import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { expect, test, vi } from 'vitest'
import UserListPage from './UserListPage'

vi.mock('../auth/AuthContext', () => ({
  useAuth: () => ({ account: { id: 'account-1', username: 'demo' }, logout: vi.fn() }),
}))

test('loads the fixed list contract and exposes the import form', async () => {
  const apiBaseUrl = import.meta.env.VITE_API_BASE_URL ?? '/api'
  const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
    new Response(JSON.stringify({ items: [], total: 0, page: 1, limit: 20 }), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    }),
  )

  render(<MemoryRouter><UserListPage /></MemoryRouter>)

  expect(screen.getByLabelText('GitHub Token')).toBeTruthy()
  expect(await screen.findByText('暂无用户，先通过 Token 导入一个吧')).toBeTruthy()
  expect(fetchMock).toHaveBeenCalledWith(
    `${apiBaseUrl}/users?page=1&limit=20`,
    expect.objectContaining({ headers: { 'Content-Type': 'application/json' } }),
  )
  fetchMock.mockRestore()
})
