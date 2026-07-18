import { request } from './client'
import type { ServiceMockResponse, ServiceName } from '../types/service'

export interface ServiceCheckDefinition {
  service: ServiceName
  label: string
  path: string
}

export const serviceChecks: readonly ServiceCheckDefinition[] = [
  { service: 'auth', label: 'Auth Service', path: '/auth/mock' },
  { service: 'profile', label: 'Profile Service', path: '/users/mock' },
  { service: 'todo', label: 'Todo Service', path: '/users/mock/todos/mock' },
]

export function checkService(definition: ServiceCheckDefinition) {
  return request<ServiceMockResponse>(definition.path)
}
