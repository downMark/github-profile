import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, test, vi } from 'vitest'
import TokenInput from './TokenInput'

const userResponse = {
  id: '8be3da22-0dea-4f5a-a681-dbeedb256ad8',
  github_id: 1,
  login: 'octocat',
  name: 'The Octocat',
  bio: null,
  avatar_url: null,
  html_url: 'https://github.com/octocat',
  public_repos: 8,
  followers: 9,
  following: 1,
  company: null,
  blog: null,
  location: null,
  github_created_at: '2011-01-25T18:44:36Z',
  created_at: '2026-07-11T00:00:00Z',
  updated_at: '2026-07-11T00:00:00Z',
}

test('submits a masked token and reports the imported user', async () => {
  const onSuccess = vi.fn()
  const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
    new Response(JSON.stringify(userResponse), {
      status: 201,
      headers: { 'Content-Type': 'application/json' },
    }),
  )
  const user = userEvent.setup()

  render(<TokenInput onSuccess={onSuccess} />)
  const input = screen.getByLabelText('GitHub Token')
  expect(input).toHaveProperty('type', 'password')
  await user.type(input, 'github-token')
  await user.click(screen.getByRole('button', { name: '导入' }))

  expect(fetchMock).toHaveBeenCalledWith(
    '/api/users',
    expect.objectContaining({ method: 'POST', body: JSON.stringify({ token: 'github-token' }) }),
  )
  expect(onSuccess).toHaveBeenCalledWith(userResponse)
  fetchMock.mockRestore()
})
