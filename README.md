1. Логику с разными очередями можно схлопнуть - подразумеваю одно устройство, поэтому достаточно одной очереди.
   (Note that it's very likely that these end up being the same queue family after all, but throughout the program 
   we will treat them as if they were separate queues for a uniform approach. Nevertheless, 
   you could add logic to explicitly prefer a physical device that supports drawing and presentation in the same 
   queue for improved performance.)

