import { InMemoryRepository } from '../src/index';
import { runRepositoryContract } from './contract';

runRepositoryContract('InMemoryRepository', (now) => new InMemoryRepository({ now }));
