package main

import (
	"context"
	"fmt"
	"github.com/go-redis/redis/v8"
	"sync"
	"testing"
)

var ctxSet = context.Background()
var rdbSet *redis.Client

func init() {
	// Initialize a new Redis connection.
	rdbSet = redis.NewClient(&redis.Options{
		Addr:     "localhost:6379",
		Password: "", // no password set
		DB:       0,  // use default DB
	})
}

func BenchmarkRedisSet(b *testing.B) {
	const maxConcurrency = 1000 // Define your level of concurrency.
	var wg sync.WaitGroup
	throttle := make(chan bool, maxConcurrency)

	for i := 0; i < b.N; i++ {
		throttle <- true
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			_, err := rdbSet.Set(ctxSet, fmt.Sprintf("key%d", i), fmt.Sprintf("value%d", i), 0).Result()
			if err != nil {
				b.Error(err)
			}
			<-throttle
		}(i)
	}
	wg.Wait()
}
