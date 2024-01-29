package main

//
//import (
//	"context"
//	"github.com/go-redis/redis/v8"
//	"sync"
//	"testing"
//)
//
//var ctx = context.Background()
//var rdb *redis.Client
//
//func init() {
//	// Initialize a new Redis connection.
//	rdb = redis.NewClient(&redis.Options{
//		Addr:     "localhost:6379",
//		Password: "", // no password set
//		DB:       0,  // use default DB
//	})
//}
//
//func BenchmarkRedisPing(b *testing.B) {
//	// Use a wait group to wait for all goroutines to finish.
//	var wg sync.WaitGroup
//
//	for i := 0; i < b.N; i++ {
//		// Increment the wait group counter.
//		wg.Add(1)
//
//		go func() {
//			defer wg.Done()
//
//			_, err := rdb.Ping(ctx).Result()
//			if err != nil {
//				b.Error(err)
//				return
//			}
//		}()
//	}
//
//	// Wait for all goroutines to finish.
//	wg.Wait()
//}
