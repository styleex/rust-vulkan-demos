1. Логику с разными очередями можно схлопнуть - подразумеваю одно устройство, поэтому достаточно одной очереди.
   (Note that it's very likely that these end up being the same queue family after all, but throughout the program 
   we will treat them as if they were separate queues for a uniform approach. Nevertheless, 
   you could add logic to explicitly prefer a physical device that supports drawing and presentation in the same 
   queue for improved performance.)

Проблемы:
1. Странный проброс wnd;
2. Клубок зависимостей модулей render_pass, pipeline, vertex и пр;
3. Заюзать аллокатор для видеопамяти;
4. Для вершинного и индексного буфера использовать один фактический буфер (рекомендация. cache-friendly);


- https://developer.nvidia.com/vulkan-memory-management
- Sparse images
- Баг тутора: один depth буфер для нескольких кадров 
  https://stackoverflow.com/questions/62371266/why-is-a-single-depth-buffer-sufficient-for-this-vulkan-swapchain-render-loop
