import { request } from './client'
import type { CreateTodoInput, Todo, TodoEventAuditListResult, TodoListResult, UpdateTodoInput } from '../types/todo'

const userTodosPath = (userId: string) => `/users/${encodeURIComponent(userId)}/todos`

export function listTodos(userId: string, page = 1, limit = 10): Promise<TodoListResult> {
  const query = new URLSearchParams({ page: String(page), limit: String(limit) })
  return request<TodoListResult>(`${userTodosPath(userId)}?${query.toString()}`)
}

export function listTodoEvents(userId: string, page = 1, limit = 10): Promise<TodoEventAuditListResult> {
  const query = new URLSearchParams({ page: String(page), limit: String(limit) })
  return request<TodoEventAuditListResult>(`${userTodosPath(userId)}/events?${query.toString()}`)
}

export function createTodo(userId: string, input: CreateTodoInput): Promise<Todo> {
  return request<Todo>(userTodosPath(userId), {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function updateTodo(userId: string, todoId: string, input: UpdateTodoInput): Promise<Todo> {
  return request<Todo>(`${userTodosPath(userId)}/${encodeURIComponent(todoId)}`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  })
}

export function deleteTodo(userId: string, todoId: string): Promise<void> {
  return request<void>(`${userTodosPath(userId)}/${encodeURIComponent(todoId)}`, {
    method: 'DELETE',
  })
}
