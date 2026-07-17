import { render, screen, waitFor } from '@testing-library/react'
import { expect, test, vi } from 'vitest'
import { AuthProvider, useAuth } from './AuthContext'

function Probe() {
  const { account, loading } = useAuth()
  return <div>{loading ? 'loading' : account?.username ?? 'anonymous'}</div>
}

test('restores the session through the refresh cookie on startup', async () => {
  const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(JSON.stringify({
    access_token: 'access-token', token_type: 'Bearer', expires_in: 900,
    account: { id: '10000000-0000-0000-0000-000000000001', username: 'demo.user' },
  }), { status: 200, headers: { 'Content-Type': 'application/json' } }))

  render(<AuthProvider><Probe /></AuthProvider>)

  expect(await screen.findByText('demo.user')).toBeTruthy()
  await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/auth/refresh', expect.objectContaining({ credentials: 'include' })))
  fetchMock.mockRestore()
})
