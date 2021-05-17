#version 450
layout(location=0) in vec3 vColor;
// `layout(location=0)` means that the value of f_color will be saved to whatever buffer is at location 0 in our application.
// In most cases, location=0 is the current texture from the swapchain aka the screen
layout(location=0) out vec4 f_color;

void main() {
    f_color = vec4(vColor, 1.0);
}