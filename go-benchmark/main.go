package main

import (
	"context"
	"fmt"
	"math/rand"
	"sync"
	"time"

	"github.com/go-redis/redis/v8"
)

var ctx = context.Background()

// Define a variable for the cumulative time taken for SET operations
var cumulativeTime time.Duration
var mutex sync.Mutex

func main() {
	rdb := redis.NewClient(&redis.Options{
		Addr:     "localhost:6379",
		Password: "",
		DB:       0,
	})

	clientsCount := 1000
	opsPerClient := 10000

	startTime := time.Now()

	var wg sync.WaitGroup
	wg.Add(clientsCount)

	for i := 0; i < clientsCount; i++ {
		go func(clientID int) {
			defer wg.Done()
			performLoadTest(rdb, opsPerClient, clientID)
		}(i)
	}

	wg.Wait()

	endTime := time.Now()
	totalTime := endTime.Sub(startTime)

	fmt.Println("Load test completed")
	fmt.Printf("Total time for operations: %v\n", totalTime)
	fmt.Printf("Cumulative time for SET operations: %v\n", cumulativeTime)
}

func performLoadTest(rdb *redis.Client, numOperations int, clientID int) {
	rand.Seed(time.Now().UnixNano())
	for i := 0; i < numOperations; i++ {
		start := time.Now()

		key := fmt.Sprintf("client%d-key%d", clientID, rand.Int())
		value := fmt.Sprintf("value%d", i)

		err := rdb.Set(ctx, key, value, 0).Err()
		if err != nil {
			//fmt.Printf("Client %d encountered an error: %v\n", clientID, err)
			continue
		}

		timeTaken := time.Since(start)

		mutex.Lock()
		cumulativeTime += timeTaken
		mutex.Unlock()

		//fmt.Printf("Client %d: Time taken for SET operation: %v\n", clientID, timeTaken)
	}
}
