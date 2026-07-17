import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import TodoPanel from './TodoPanel'
import { createTodo, deleteTodo, listTodos, updateTodo } from '../api/todos'

vi.mock('../api/todos', () => ({
  listTodos: vi.fn(),
  createTodo: vi.fn(),
  updateTodo: vi.fn(),
  deleteTodo: vi.fn(),
}))

const todo = {
  id: '20000000-0000-0000-0000-000000000001',
  github_user_id: '10000000-0000-0000-0000-000000000001',
  title: 'Write tests',
  description: 'Todo panel',
  completed: false,
  created_at: '2026-07-14T00:00:00Z',
  updated_at: '2026-07-14T00:00:00Z',
}

describe('TodoPanel', () => {
  beforeEach(() => {
    vi.mocked(listTodos).mockReset().mockResolvedValue({ items: [todo], total: 1, page: 1, limit: 10 })
    vi.mocked(createTodo).mockReset().mockResolvedValue(todo)
    vi.mocked(updateTodo).mockReset().mockResolvedValue({ ...todo, completed: true })
    vi.mocked(deleteTodo).mockReset().mockResolvedValue(undefined)
  })

  it('loads reachable todos and toggles completion', async () => {
    const user = userEvent.setup()
    render(<TodoPanel userId={todo.github_user_id} />)
    expect(await screen.findByText('Write tests')).toBeTruthy()
    await user.click(screen.getByRole('checkbox'))
    expect(updateTodo).toHaveBeenCalledWith(todo.github_user_id, todo.id, { completed: true })
  })

  it('creates a todo and reloads the first page', async () => {
    const user = userEvent.setup()
    render(<TodoPanel userId={todo.github_user_id} />)
    await screen.findByText('Write tests')
    await user.type(screen.getByLabelText('标题'), 'Ship Go service')
    await user.click(screen.getByRole('button', { name: '添加 Todo' }))
    await waitFor(() => expect(createTodo).toHaveBeenCalledWith(todo.github_user_id, {
      title: 'Ship Go service',
      description: null,
    }))
    expect(listTodos).toHaveBeenLastCalledWith(todo.github_user_id, 1, 10)
  })

  it('edits and deletes a todo', async () => {
    const user = userEvent.setup()
    vi.mocked(updateTodo).mockResolvedValueOnce({
      ...todo,
      title: 'Updated title',
      description: null,
    })
    render(<TodoPanel userId={todo.github_user_id} />)
    await screen.findByText('Write tests')

    await user.click(screen.getByRole('button', { name: '编辑' }))
    const titleInput = screen.getByLabelText('编辑标题')
    await user.clear(titleInput)
    await user.type(titleInput, 'Updated title')
    await user.clear(screen.getByLabelText('编辑描述'))
    await user.click(screen.getByRole('button', { name: '保存' }))

    await waitFor(() => expect(updateTodo).toHaveBeenCalledWith(todo.github_user_id, todo.id, {
      title: 'Updated title',
      description: null,
    }))
    expect(await screen.findByText('Updated title')).toBeTruthy()

    await user.click(screen.getByRole('button', { name: '删除' }))
    await waitFor(() => expect(deleteTodo).toHaveBeenCalledWith(todo.github_user_id, todo.id))
    expect(listTodos).toHaveBeenLastCalledWith(todo.github_user_id, 1, 10)
  })

  it('loads the next page', async () => {
    const user = userEvent.setup()
    vi.mocked(listTodos)
      .mockResolvedValueOnce({ items: [todo], total: 11, page: 1, limit: 10 })
      .mockResolvedValueOnce({ items: [], total: 11, page: 2, limit: 10 })
    render(<TodoPanel userId={todo.github_user_id} />)
    await screen.findByText('Write tests')
    await user.click(screen.getByRole('button', { name: '下一页' }))
    await waitFor(() => expect(listTodos).toHaveBeenLastCalledWith(todo.github_user_id, 2, 10))
  })
})
